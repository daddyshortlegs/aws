use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod config;
mod ip_lookup;
mod proxy_service;
mod registry;

use config::Config;
use proxy_service::ProxyService;
use registry::BackendRegistry;

// ── OpenAPI schema types ──────────────────────────────────────────────────────
// These mirror the request/response structs defined in the backend. They exist
// solely so utoipa can generate schema definitions for the Swagger UI — the
// proxy itself forwards JSON opaquely and never constructs these types at runtime.

/// Request body for launching a new VM.
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
struct LaunchVmRequest {
    /// Human-readable name for the VM.
    name: String,
    instance_type: String,
    region: String,
}

/// Response returned after a VM launch attempt.
#[derive(serde::Serialize, utoipa::ToSchema)]
struct LaunchVmResponse {
    success: bool,
    message: String,
    /// UUID of the newly created VM. Present on success.
    instance_id: Option<String>,
    /// SSH host to connect to. Empty in bridge mode until a DHCP lease is assigned.
    ssh_host: Option<String>,
    ssh_port: Option<u16>,
    pid: Option<u32>,
}

/// A single VM entry as returned by `/list-vms`.
#[derive(serde::Serialize, utoipa::ToSchema)]
struct VmListEntry {
    id: String,
    name: String,
    /// SSH host to connect to. Resolved from the dnsmasq lease file by the proxy in bridge mode.
    ssh_host: String,
    ssh_port: u16,
    pid: u32,
    /// Whether the QEMU process is currently alive on the worker.
    running: bool,
    /// MAC address of the VM's network interface. Present in bridge mode only.
    #[serde(skip_serializing_if = "Option::is_none")]
    mac_address: Option<String>,
}

/// Request body for gracefully stopping a running VM.
#[derive(serde::Deserialize, utoipa::ToSchema)]
struct StopVmRequest {
    /// UUID of the VM to stop.
    id: String,
}

/// Request body for starting (resuming) a stopped VM.
#[derive(serde::Deserialize, utoipa::ToSchema)]
struct StartVmRequest {
    /// UUID of the VM to start.
    id: String,
}

/// Request body for creating a new volume.
#[derive(serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
struct LaunchVolumeRequest {
    /// Human-readable name for the volume.
    name: String,
    /// Size of the volume in gigabytes.
    size_gb: u64,
}

/// Response returned after a volume creation attempt.
#[derive(serde::Serialize, utoipa::ToSchema)]
struct LaunchVolumeResponse {
    success: bool,
    message: String,
    /// UUID of the newly created volume. Present on success.
    id: Option<String>,
    name: Option<String>,
    /// Absolute path where the volume is mounted on the worker node.
    mount_path: Option<String>,
}

/// A single volume entry as returned by `/list-volumes`.
#[derive(serde::Serialize, utoipa::ToSchema)]
struct VolumeInfo {
    id: String,
    name: String,
    mount_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    loop_device: Option<String>,
}

/// Request body for deleting a volume.
#[derive(serde::Deserialize, utoipa::ToSchema)]
struct DeleteVolumeRequest {
    /// UUID of the volume to delete.
    id: String,
}

/// A single file or directory entry within a volume.
#[derive(serde::Serialize, utoipa::ToSchema)]
struct VolumeFileEntry {
    name: String,
    is_dir: bool,
    size_bytes: u64,
    /// Last-modified time as seconds since the Unix epoch.
    modified_secs: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub proxy_service: Arc<ProxyService>,
    pub registry: Arc<RwLock<BackendRegistry>>,
    /// Path to the JSON file used to persist vm_id → backend_url across restarts.
    pub vm_backends_file: PathBuf,
    /// Path to the JSON file used to persist volume_id → backend_url across restarts.
    pub volume_backends_file: PathBuf,
}

/// Load a vm_id → backend_url map from a JSON file. Returns an empty map if
/// the file is absent or unreadable — this is normal on first startup.
async fn load_vm_backends(path: &Path) -> HashMap<String, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Persist the current volume_id → backend_url map to disk. Logs a warning on
/// failure rather than propagating an error — a failed write is non-fatal.
async fn save_volume_backends(path: &Path, backends: &HashMap<String, String>) {
    match serde_json::to_string(backends) {
        Ok(content) => {
            if let Err(e) = tokio::fs::write(path, content).await {
                tracing::warn!("Failed to persist volume_backends to {path:?}: {e}");
            }
        }
        Err(e) => tracing::error!("Failed to serialize volume_backends: {e}"),
    }
}

/// Load a volume_id → backend_url map from a JSON file. Returns an empty map if
/// the file is absent or unreadable — this is normal on first startup.
async fn load_volume_backends(path: &Path) -> HashMap<String, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

/// Persist the current vm_id → backend_url map to disk. Logs a warning on
/// failure rather than propagating an error — a failed write is non-fatal.
async fn save_vm_backends(path: &Path, backends: &HashMap<String, String>) {
    match serde_json::to_string(backends) {
        Ok(content) => {
            if let Err(e) = tokio::fs::write(path, content).await {
                tracing::warn!("Failed to persist vm_backends to {path:?}: {e}");
            }
        }
        Err(e) => tracing::error!("Failed to serialize vm_backends: {e}"),
    }
}

#[utoipa::path(
    post,
    path = "/launch-volume",
    request_body = LaunchVolumeRequest,
    responses(
        (status = 200, description = "Volume created and mounted", body = LaunchVolumeResponse),
        (status = 500, description = "Failed to create or mount volume", body = LaunchVolumeResponse),
        (status = 503, description = "No backend worker is registered"),
    ),
    tag = "volumes"
)]
async fn launch_volume_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let backend_url = match state.registry.write().await.round_robin_url() {
        Some(u) => u,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Backend not yet registered",
            )
                .into_response();
        }
    };

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body = Some(request.into_body());

    let response = state
        .proxy_service
        .proxy_request_to(backend_url.clone(), method, uri, headers, body, None)
        .await;

    if response.status().is_success() {
        let (parts, resp_body) = response.into_parts();
        let bytes = axum::body::to_bytes(resp_body, usize::MAX)
            .await
            .unwrap_or_default();
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(volume_id) = val.get("id").and_then(|v| v.as_str()) {
                state
                    .registry
                    .write()
                    .await
                    .register_volume(volume_id.to_string(), backend_url);
                let backends = state.registry.read().await.all_volume_backends();
                save_volume_backends(&state.volume_backends_file, &backends).await;
            }
        }
        return Response::from_parts(parts, Body::from(bytes)).into_response();
    }

    response.into_response()
}

#[utoipa::path(
    get,
    path = "/list-volumes",
    responses(
        (status = 200, description = "Aggregated list of all volumes across all backends", body = Vec<VolumeInfo>),
        (status = 503, description = "No backend worker is registered"),
    ),
    tag = "volumes"
)]
async fn list_volumes_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let headers = request.headers().clone();
    let uri = request.uri().clone();
    state.proxy_service.list_all(uri.path(), headers).await
}

#[utoipa::path(
    delete,
    path = "/delete-volume",
    request_body = DeleteVolumeRequest,
    responses(
        (status = 200, description = "Volume unmounted and removed"),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "Volume ID not known to this proxy"),
    ),
    tag = "volumes"
)]
async fn delete_volume_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    let volume_id = match serde_json::from_slice::<DeleteVolumeRequest>(&bytes) {
        Ok(req) => req.id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid request body").into_response(),
    };

    let backend_url = match state.registry.read().await.backend_for_volume(&volume_id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Unknown volume ID").into_response(),
    };

    let response = state
        .proxy_service
        .proxy_request_to(
            backend_url,
            parts.method,
            parts.uri,
            parts.headers,
            Some(Body::from(bytes)),
            None,
        )
        .await;

    if response.status().is_success() {
        state.registry.write().await.remove_volume(&volume_id);
        let backends = state.registry.read().await.all_volume_backends();
        save_volume_backends(&state.volume_backends_file, &backends).await;
    }

    response.into_response()
}

#[utoipa::path(
    get,
    path = "/volume-files/{id}",
    params(
        ("id" = String, Path, description = "Volume UUID")
    ),
    responses(
        (status = 200, description = "List of files in the volume root", body = Vec<VolumeFileEntry>),
        (status = 404, description = "Volume ID not known to this proxy"),
    ),
    tag = "volumes"
)]
async fn list_volume_files_handler(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    let backend_url = match state.registry.read().await.backend_for_volume(&id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Unknown volume ID").into_response(),
    };

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    state
        .proxy_service
        .proxy_request_to(backend_url, method, uri, headers, None, None)
        .await
        .into_response()
}

#[utoipa::path(
    post,
    path = "/stop-vm",
    request_body = StopVmRequest,
    responses(
        (status = 200, description = "ACPI shutdown signal sent; VM is powering off"),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "VM ID not known to this proxy"),
        (status = 409, description = "VM is not currently running"),
    ),
    tag = "vms"
)]
/// Route /stop-vm to the backend that owns the VM. The backend sends
/// `system_powerdown` via the QEMU monitor for a graceful ACPI shutdown.
async fn stop_vm_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    let vm_id = match serde_json::from_slice::<StopVmRequest>(&bytes) {
        Ok(req) => req.id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid request body").into_response(),
    };

    let backend_url = match state.registry.read().await.backend_for_vm(&vm_id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Unknown VM ID").into_response(),
    };

    state
        .proxy_service
        .proxy_request_to(
            backend_url,
            parts.method,
            parts.uri,
            parts.headers,
            Some(Body::from(bytes)),
            None,
        )
        .await
        .into_response()
}

#[utoipa::path(
    post,
    path = "/start-vm",
    request_body = StartVmRequest,
    responses(
        (status = 200, description = "VM re-launched from its existing disk image"),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "VM ID not known to this proxy"),
        (status = 409, description = "VM is already running"),
    ),
    tag = "vms"
)]
/// Route /start-vm to the backend that owns the VM. The backend re-launches
/// the QEMU process from the existing qcow2 image.
async fn start_vm_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    let vm_id = match serde_json::from_slice::<StartVmRequest>(&bytes) {
        Ok(req) => req.id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid request body").into_response(),
    };

    let backend_url = match state.registry.read().await.backend_for_vm(&vm_id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Unknown VM ID").into_response(),
    };

    state
        .proxy_service
        .proxy_request_to(
            backend_url,
            parts.method,
            parts.uri,
            parts.headers,
            Some(Body::from(bytes)),
            None,
        )
        .await
        .into_response()
}

#[derive(OpenApi)]
#[openapi(
    paths(
        registry::register_handler,
        launch_vm_handler,
        list_vms_handler,
        delete_vm_handler,
        stop_vm_handler,
        start_vm_handler,
        launch_volume_handler,
        list_volumes_handler,
        delete_volume_handler,
        list_volume_files_handler,
    ),
    components(schemas(
        registry::RegisterRequest,
        registry::RegisterResponse,
        LaunchVmRequest,
        LaunchVmResponse,
        VmListEntry,
        DeleteVmRequest,
        StopVmRequest,
        StartVmRequest,
        LaunchVolumeRequest,
        LaunchVolumeResponse,
        VolumeInfo,
        DeleteVolumeRequest,
        VolumeFileEntry,
    )),
    tags(
        (name = "vms", description = "VM lifecycle management"),
        (name = "volumes", description = "Volume lifecycle management"),
        (name = "internal", description = "Backend registration — called by worker nodes on startup, not by end users"),
    ),
    info(title = "Andy's Web Services API", version = "0.1.0")
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    let config = Config::load().expect("Failed to load configuration");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.log_level))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting proxy server with config: {:?}", config);

    let registry: Arc<RwLock<BackendRegistry>> = Arc::new(RwLock::new(BackendRegistry::new()));

    // Restore vm_id → backend_url mappings saved before the last proxy restart.
    let saved_vms = load_vm_backends(&config.vm_backends_file).await;
    let vm_count = saved_vms.len();
    {
        let mut reg = registry.write().await;
        for (vm_id, backend_url) in saved_vms {
            reg.register_vm(vm_id, backend_url);
        }
    }
    if vm_count > 0 {
        tracing::info!(
            "Restored {vm_count} VM-backend mapping(s) from {:?}",
            config.vm_backends_file
        );
    }

    // Restore volume_id → backend_url mappings saved before the last proxy restart.
    let saved_volumes = load_volume_backends(&config.volume_backends_file).await;
    let volume_count = saved_volumes.len();
    {
        let mut reg = registry.write().await;
        for (volume_id, backend_url) in saved_volumes {
            reg.register_volume(volume_id, backend_url);
        }
    }
    if volume_count > 0 {
        tracing::info!(
            "Restored {volume_count} volume-backend mapping(s) from {:?}",
            config.volume_backends_file
        );
    }

    let proxy_service = Arc::new(ProxyService::new(
        Arc::clone(&registry),
        config.lease_file.clone(),
    ));
    let state = AppState {
        proxy_service,
        registry,
        vm_backends_file: config.vm_backends_file.clone(),
        volume_backends_file: config.volume_backends_file.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_router = Router::new()
        .route("/register", post(registry::register_handler))
        .route("/launch-vm", post(launch_vm_handler))
        .route("/list-vms", get(list_vms_handler))
        .route("/delete-vm", delete(delete_vm_handler))
        .route("/stop-vm", post(stop_vm_handler))
        .route("/start-vm", post(start_vm_handler))
        .route("/launch-volume", post(launch_volume_handler))
        .route("/list-volumes", get(list_volumes_handler))
        .route("/delete-volume", delete(delete_volume_handler))
        .route("/volume-files/:id", get(list_volume_files_handler))
        .fallback(proxy_handler)
        .with_state(state);

    let app = api_router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.listen_ip, config.proxy_port))
            .await
            .unwrap();

    tracing::info!(
        "Proxy server listening on {}",
        listener.local_addr().unwrap()
    );
    tracing::info!("Waiting for backend to register via POST /register");

    axum::serve(listener, app).await.unwrap();
}

#[utoipa::path(
    post,
    path = "/launch-vm",
    request_body = LaunchVmRequest,
    responses(
        (status = 200, description = "VM launched successfully", body = LaunchVmResponse),
        (status = 503, description = "No backend worker is registered"),
    ),
    tag = "vms"
)]
/// Round-robin /launch-vm: selects the next backend, forwards the request,
/// and if the backend accepts it records the vm_id → backend mapping.
async fn launch_vm_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let backend_url = match state.registry.write().await.round_robin_url() {
        Some(u) => u,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Backend not yet registered",
            )
                .into_response();
        }
    };

    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body = Some(request.into_body());

    let response = state
        .proxy_service
        .proxy_request_to(backend_url.clone(), method, uri, headers, body, None)
        .await;

    // Only record the VM→backend mapping if the launch succeeded.
    if response.status().is_success() {
        let (parts, resp_body) = response.into_parts();
        let bytes = axum::body::to_bytes(resp_body, usize::MAX)
            .await
            .unwrap_or_default();
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(vm_id) = val.get("instance_id").and_then(|v| v.as_str()) {
                state
                    .registry
                    .write()
                    .await
                    .register_vm(vm_id.to_string(), backend_url);
                let backends = state.registry.read().await.all_vm_backends();
                save_vm_backends(&state.vm_backends_file, &backends).await;
            }
        }
        return Response::from_parts(parts, Body::from(bytes)).into_response();
    }

    response.into_response()
}

#[utoipa::path(
    get,
    path = "/list-vms",
    responses(
        (status = 200, description = "Aggregated list of all running VMs across all backends", body = Vec<VmListEntry>),
        (status = 503, description = "No backend worker is registered"),
    ),
    tag = "vms"
)]
/// Fan-out /list-vms to all backends and return the merged JSON array.
async fn list_vms_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let headers = request.headers().clone();
    let uri = request.uri().clone();
    state.proxy_service.list_all(uri.path(), headers).await
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct DeleteVmRequest {
    /// UUID of the VM to delete.
    id: String,
}

#[utoipa::path(
    delete,
    path = "/delete-vm",
    request_body = DeleteVmRequest,
    responses(
        (status = 200, description = "VM terminated and removed"),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "VM ID not known to this proxy"),
    ),
    tag = "vms"
)]
/// Route /delete-vm to the backend that owns the VM, identified by the ID in
/// the request body. Returns 400 if the body is invalid JSON, 404 if the VM
/// is not known to the registry.
async fn delete_vm_handler(
    State(state): State<AppState>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (parts, body) = request.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();

    let vm_id = match serde_json::from_slice::<DeleteVmRequest>(&bytes) {
        Ok(req) => req.id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid request body").into_response(),
    };

    let backend_url = match state.registry.read().await.backend_for_vm(&vm_id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, "Unknown VM ID").into_response(),
    };

    let response = state
        .proxy_service
        .proxy_request_to(
            backend_url,
            parts.method,
            parts.uri,
            parts.headers,
            Some(Body::from(bytes)),
            None,
        )
        .await;

    if response.status().is_success() {
        state.registry.write().await.remove_vm(&vm_id);
        let backends = state.registry.read().await.all_vm_backends();
        save_vm_backends(&state.vm_backends_file, &backends).await;
    }

    response.into_response()
}

async fn proxy_handler(State(state): State<AppState>, request: Request<Body>) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    let body = Some(request.into_body());

    state
        .proxy_service
        .proxy_request(method, uri, headers, body, None)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    // ── helpers ───────────────────────────────────────────────────────────────

    async fn start_mock_backend(status: u16, body: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let app = Router::new().fallback(move || async move {
            (
                axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::OK),
                body,
            )
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        port
    }

    /// Backend that counts requests and returns a fixed body.
    async fn start_counting_backend(body: &'static str) -> (u16, Arc<Mutex<usize>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let count = Arc::new(Mutex::new(0usize));
        let c = count.clone();
        let app = Router::new().fallback(move || {
            let c = c.clone();
            async move {
                *c.lock().await += 1;
                (axum::http::StatusCode::OK, body)
            }
        });
        tokio::spawn(async move { axum::serve(listener, app).await.ok() });
        tokio::task::yield_now().await;
        (port, count)
    }

    fn build_test_app() -> (Router, Arc<RwLock<BackendRegistry>>) {
        // Use unique temp paths per call so parallel tests never share files.
        let vm_backends_file =
            std::env::temp_dir().join(format!("test-vm-backends-{}.json", uuid::Uuid::new_v4()));
        let volume_backends_file = std::env::temp_dir().join(format!(
            "test-volume-backends-{}.json",
            uuid::Uuid::new_v4()
        ));
        build_test_app_with_files(vm_backends_file, volume_backends_file)
    }

    fn build_test_app_with_files(
        vm_backends_file: PathBuf,
        volume_backends_file: PathBuf,
    ) -> (Router, Arc<RwLock<BackendRegistry>>) {
        let registry: Arc<RwLock<BackendRegistry>> = Arc::new(RwLock::new(BackendRegistry::new()));
        let proxy_service = Arc::new(ProxyService::new(
            Arc::clone(&registry),
            PathBuf::from("/nonexistent/leases"),
        ));
        let state = AppState {
            proxy_service,
            registry: Arc::clone(&registry),
            vm_backends_file,
            volume_backends_file,
        };

        let cors = tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any);

        let app = Router::new()
            .route("/register", post(registry::register_handler))
            .route("/launch-vm", post(launch_vm_handler))
            .route("/list-vms", get(list_vms_handler))
            .route("/delete-vm", delete(delete_vm_handler))
            .route("/stop-vm", post(stop_vm_handler))
            .route("/start-vm", post(start_vm_handler))
            .route("/launch-volume", post(launch_volume_handler))
            .route("/list-volumes", get(list_volumes_handler))
            .route("/delete-volume", delete(delete_volume_handler))
            .route("/volume-files/:id", get(list_volume_files_handler))
            .fallback(proxy_handler)
            .layer(cors)
            .with_state(state);

        (app, registry)
    }

    async fn body_string(resp: axum::http::Response<Body>) -> String {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    // ── register handler ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_register_sets_backend_url() {
        let (app, registry) = build_test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/register")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"ip":"127.0.0.1","port":8081}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let body = body_string(resp).await;
        let val: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(val["id"].is_string());

        let reg = registry.read().await;
        assert_eq!(reg.any_url().as_deref(), Some("http://127.0.0.1:8081"));
    }

    // ── proxy routing ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_proxy_before_registration_returns_503() {
        let (app, _) = build_test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/list-vms")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_full_proxy_flow_register_then_request() {
        // list-vms fan-out aggregates JSON arrays; mock must return a valid array.
        let port = start_mock_backend(200, r#"[{"id":"vm1"}]"#).await;
        let (app, _) = build_test_app();

        let register_req = Request::builder()
            .method("POST")
            .uri("/register")
            .header("content-type", "application/json")
            .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
            .unwrap();
        app.clone().oneshot(register_req).await.unwrap();

        let proxy_req = Request::builder()
            .method("GET")
            .uri("/list-vms")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(proxy_req).await.unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let body = body_string(resp).await;
        let val: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert!(val.is_array());
        assert!(!val.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_launch_vm_route_accepts_post() {
        // Backend returns a 201 with a non-JSON body; the handler still proxies it.
        let port = start_mock_backend(201, "launched").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/launch-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"test-vm"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_delete_vm_unknown_id_returns_404() {
        let port = start_mock_backend(200, "deleted").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"unknown-vm-id"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_fallback_route_proxies_arbitrary_path() {
        let port = start_mock_backend(200, "arbitrary").await;
        let (app, _) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/some/unknown/path")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(body_string(resp).await, "arbitrary");
    }

    #[tokio::test]
    async fn test_cors_headers_present_on_response() {
        let (app, _) = build_test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/list-vms")
                    .header("origin", "http://localhost:3000")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            resp.headers().contains_key("access-control-allow-origin"),
            "CORS allow-origin header must be present"
        );
    }

    // ── multi-backend routing ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_launch_vm_round_robins_across_backends() {
        let (port_a, count_a) = start_counting_backend(r#"{"instance_id":"vm-a"}"#).await;
        let (port_b, count_b) = start_counting_backend(r#"{"instance_id":"vm-b"}"#).await;
        let (app, _) = build_test_app();

        // Register both backends.
        for port in [port_a, port_b] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/register")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Two launches should alternate across the two backends.
        for _ in 0..2 {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/launch-vm")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"name":"test"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        assert_eq!(
            *count_a.lock().await,
            1,
            "backend A should receive exactly 1 launch"
        );
        assert_eq!(
            *count_b.lock().await,
            1,
            "backend B should receive exactly 1 launch"
        );
    }

    #[tokio::test]
    async fn test_list_vms_aggregates_all_backends() {
        let port_a = start_mock_backend(200, r#"[{"id":"vm-a"}]"#).await;
        let port_b = start_mock_backend(200, r#"[{"id":"vm-b"}]"#).await;
        let (app, _) = build_test_app();

        for port in [port_a, port_b] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/register")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/list-vms")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let body = body_string(resp).await;
        let val: serde_json::Value = serde_json::from_str(&body).unwrap();
        let arr = val.as_array().unwrap();
        assert_eq!(
            arr.len(),
            2,
            "merged list should contain VMs from both backends"
        );
        let ids: Vec<&str> = arr.iter().filter_map(|v| v["id"].as_str()).collect();
        assert!(ids.contains(&"vm-a"));
        assert!(ids.contains(&"vm-b"));
    }

    #[tokio::test]
    async fn test_delete_vm_routes_to_correct_backend() {
        // Backend A records whether it received a DELETE; backend B does too.
        let delete_count_a = Arc::new(Mutex::new(0usize));
        let delete_count_b = Arc::new(Mutex::new(0usize));

        // Each backend counts every request (launch + delete).
        let dca = delete_count_a.clone();
        let listener_da = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port_da = listener_da.local_addr().unwrap().port();
        tokio::spawn(async move {
            let app = Router::new().fallback(move || {
                let dca = dca.clone();
                async move {
                    *dca.lock().await += 1;
                    (axum::http::StatusCode::OK, r#"{"instance_id":"vm-a"}"#)
                }
            });
            axum::serve(listener_da, app).await.ok()
        });

        let dcb = delete_count_b.clone();
        let listener_db = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port_db = listener_db.local_addr().unwrap().port();
        tokio::spawn(async move {
            let app = Router::new().fallback(move || {
                let dcb = dcb.clone();
                async move {
                    *dcb.lock().await += 1;
                    (axum::http::StatusCode::OK, r#"{"instance_id":"vm-b"}"#)
                }
            });
            axum::serve(listener_db, app).await.ok()
        });
        tokio::task::yield_now().await;

        let (app, _) = build_test_app();

        // Register the delete-tracking backends (they also handle launch).
        for port in [port_da, port_db] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/register")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Launch twice so vm-a maps to backend A and vm-b maps to backend B.
        for _ in 0..2 {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/launch-vm")
                        .header("content-type", "application/json")
                        .body(Body::from(r#"{"name":"test"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        // Delete vm-b — should route only to backend B (port_db).
        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"vm-b"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        // Backend A must not have received an extra request for the delete.
        assert_eq!(
            *delete_count_a.lock().await,
            1,
            "backend A should have 1 hit (launch only)"
        );
        // Backend B received the launch AND the delete.
        assert_eq!(
            *delete_count_b.lock().await,
            2,
            "backend B should have 2 hits (launch + delete)"
        );
    }

    // ── persistence ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_load_vm_backends_missing_file_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.json");
        let map = load_vm_backends(&path).await;
        assert!(map.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_load_vm_backends_round_trips() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("backends.json");

        let mut map = HashMap::new();
        map.insert("vm-a".to_string(), "http://10.0.0.1:8081".to_string());
        map.insert("vm-b".to_string(), "http://10.0.0.2:8082".to_string());

        save_vm_backends(&path, &map).await;

        let loaded = load_vm_backends(&path).await;
        assert_eq!(loaded, map);
    }

    #[tokio::test]
    async fn test_load_vm_backends_invalid_json_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("bad.json");
        tokio::fs::write(&path, b"not valid json").await.unwrap();
        let map = load_vm_backends(&path).await;
        assert!(map.is_empty());
    }

    #[tokio::test]
    async fn test_launch_vm_persists_mapping_to_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let backends_file = tmp.path().join("vm-backends.json");
        let (app, _) =
            build_test_app_with_files(backends_file.clone(), tmp.path().join("vol.json"));

        // Start a mock backend that returns a successful launch response.
        let port = start_mock_backend(200, r#"{"success":true,"instance_id":"vm-xyz"}"#).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/launch-vm")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

        let persisted = load_vm_backends(&backends_file).await;
        assert!(
            persisted.contains_key("vm-xyz"),
            "vm-xyz should be in the persisted file"
        );
    }

    #[tokio::test]
    async fn test_delete_vm_removes_mapping_from_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let backends_file = tmp.path().join("vm-backends.json");
        let (app, registry) =
            build_test_app_with_files(backends_file.clone(), tmp.path().join("vol.json"));

        // Pre-populate the registry and the file with a known mapping.
        let port = start_mock_backend(200, "deleted").await;
        registry.write().await.register_vm(
            "vm-to-delete".to_string(),
            format!("http://127.0.0.1:{port}"),
        );

        let mut initial = HashMap::new();
        initial.insert(
            "vm-to-delete".to_string(),
            format!("http://127.0.0.1:{port}"),
        );
        save_vm_backends(&backends_file, &initial).await;

        // Register the backend so the proxy can route to it.
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"vm-to-delete"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), axum::http::StatusCode::OK);

        let persisted = load_vm_backends(&backends_file).await;
        assert!(
            !persisted.contains_key("vm-to-delete"),
            "vm-to-delete should be removed from the persisted file"
        );
    }

    // ── volume handlers ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_launch_volume_records_backend_mapping() {
        let port = start_mock_backend(200, r#"{"success":true,"id":"vol-abc"}"#).await;
        let (app, registry) = build_test_app();

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/launch-volume")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"data","size_gb":10}"#))
                .unwrap(),
        )
        .await
        .unwrap();

        assert!(
            registry
                .read()
                .await
                .backend_for_volume("vol-abc")
                .is_some(),
            "volume-backend mapping should be recorded after launch"
        );
    }

    #[tokio::test]
    async fn test_launch_volume_persists_mapping_to_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let vol_file = tmp.path().join("volume-backends.json");
        let (app, _) = build_test_app_with_files(tmp.path().join("vm.json"), vol_file.clone());

        let port = start_mock_backend(200, r#"{"success":true,"id":"vol-xyz"}"#).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/launch-volume")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name":"data","size_gb":5}"#))
                .unwrap(),
        )
        .await
        .unwrap();

        let persisted = load_volume_backends(&vol_file).await;
        assert!(
            persisted.contains_key("vol-xyz"),
            "vol-xyz should be in the persisted volume-backends file"
        );
    }

    #[tokio::test]
    async fn test_list_volumes_aggregates_all_backends() {
        let port_a =
            start_mock_backend(200, r#"[{"id":"vol-a","name":"a","mount_path":"/mnt/a"}]"#).await;
        let port_b =
            start_mock_backend(200, r#"[{"id":"vol-b","name":"b","mount_path":"/mnt/b"}]"#).await;
        let (app, _) = build_test_app();

        for port in [port_a, port_b] {
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/register")
                        .header("content-type", "application/json")
                        .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                        .unwrap(),
                )
                .await
                .unwrap();
        }

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/list-volumes")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        let arr: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_delete_volume_unknown_id_returns_404() {
        let (app, _) = build_test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-volume")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"no-such-volume"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_volume_routes_to_owning_backend_and_removes_mapping() {
        let tmp = tempfile::TempDir::new().unwrap();
        let vol_file = tmp.path().join("volume-backends.json");
        let (app, registry) =
            build_test_app_with_files(tmp.path().join("vm.json"), vol_file.clone());

        let port = start_mock_backend(200, "deleted").await;
        registry.write().await.register_volume(
            "vol-to-delete".to_string(),
            format!("http://127.0.0.1:{port}"),
        );

        let mut initial = HashMap::new();
        initial.insert(
            "vol-to-delete".to_string(),
            format!("http://127.0.0.1:{port}"),
        );
        save_volume_backends(&vol_file, &initial).await;

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/delete-volume")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"vol-to-delete"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let persisted = load_volume_backends(&vol_file).await;
        assert!(
            !persisted.contains_key("vol-to-delete"),
            "vol-to-delete should be removed from the persisted file"
        );
    }

    #[tokio::test]
    async fn test_list_volume_files_unknown_id_returns_404() {
        let (app, _) = build_test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/volume-files/no-such-id")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_volume_files_routes_to_owning_backend() {
        let port = start_mock_backend(
            200,
            r#"[{"name":"file.txt","is_dir":false,"size_bytes":5,"modified_secs":0}]"#,
        )
        .await;
        let (app, registry) = build_test_app();

        registry
            .write()
            .await
            .register_volume("my-vol".to_string(), format!("http://127.0.0.1:{port}"));

        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/register")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"ip":"127.0.0.1","port":{port}}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/volume-files/my-vol")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_string(resp).await;
        let arr: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1);
    }

    // ── stop/start vm handlers ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_stop_vm_unknown_id_returns_404() {
        let (app, _) = build_test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stop-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"no-such-vm"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_stop_vm_routes_to_owning_backend() {
        let port = start_mock_backend(200, "stopped").await;
        let (app, registry) = build_test_app();

        registry
            .write()
            .await
            .register_vm("vm-to-stop".to_string(), format!("http://127.0.0.1:{port}"));

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/stop-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"vm-to-stop"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_start_vm_unknown_id_returns_404() {
        let (app, _) = build_test_app();

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/start-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"no-such-vm"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_start_vm_routes_to_owning_backend() {
        let port = start_mock_backend(200, "started").await;
        let (app, registry) = build_test_app();

        registry.write().await.register_vm(
            "vm-to-start".to_string(),
            format!("http://127.0.0.1:{port}"),
        );

        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/start-vm")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"id":"vm-to-start"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }
}
