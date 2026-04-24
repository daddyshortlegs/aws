use std::collections::HashMap;
use uuid::Uuid;

pub struct BackendEntry {
    pub url: String,
}

#[derive(Default)]
pub struct BackendRegistry {
    backends: HashMap<Uuid, BackendEntry>,
    /// Insertion order for deterministic round-robin iteration.
    order: Vec<Uuid>,
    /// Round-robin cursor; wraps with `wrapping_add` to avoid overflow.
    next_index: usize,
    /// Maps vm_id → backend_url so /delete-vm can route to the owning backend.
    vm_backends: HashMap<String, String>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a backend and return its assigned UUID.
    /// If the same ip:port is already registered, returns the existing UUID
    /// without adding a duplicate entry.
    pub fn register(&mut self, ip: &str, port: u16) -> Uuid {
        let url = format!("http://{ip}:{port}");
        if let Some(id) = self
            .order
            .iter()
            .find(|id| self.backends.get(id).is_some_and(|e| e.url == url))
        {
            return *id;
        }

        let id = Uuid::new_v4();
        self.backends.insert(id, BackendEntry { url });
        self.order.push(id);
        id
    }

    /// Advance the round-robin cursor and return the next backend URL.
    /// Returns `None` if no backends are registered.
    pub fn round_robin_url(&mut self) -> Option<String> {
        if self.order.is_empty() {
            return None;
        }
        let id = self.order[self.next_index % self.order.len()];
        self.next_index = self.next_index.wrapping_add(1);
        self.backends.get(&id).map(|e| e.url.clone())
    }

    /// Return any registered backend URL (first registered; used by fallback routes).
    pub fn any_url(&self) -> Option<String> {
        self.order
            .first()
            .and_then(|id| self.backends.get(id))
            .map(|e| e.url.clone())
    }

    /// Return all registered backend URLs in registration order (used by /list-vms fan-out).
    pub fn all_urls(&self) -> Vec<String> {
        self.order
            .iter()
            .filter_map(|id| self.backends.get(id))
            .map(|e| e.url.clone())
            .collect()
    }

    /// Record which backend a VM was launched on.
    pub fn register_vm(&mut self, vm_id: String, backend_url: String) {
        self.vm_backends.insert(vm_id, backend_url);
    }

    /// Look up the backend URL that owns a given VM.
    pub fn backend_for_vm(&self, vm_id: &str) -> Option<String> {
        self.vm_backends.get(vm_id).cloned()
    }

    /// Remove the backend mapping for a VM (called after a successful delete).
    pub fn remove_vm(&mut self, vm_id: &str) {
        self.vm_backends.remove(vm_id);
    }

    /// Return a snapshot of all vm_id → backend_url mappings, for persistence.
    pub fn all_vm_backends(&self) -> HashMap<String, String> {
        self.vm_backends.clone()
    }

    /// Test helper: create a registry pre-populated with a single known URL.
    #[cfg(test)]
    pub fn with_url(url: String) -> Self {
        let id = Uuid::new_v4();
        let mut backends = HashMap::new();
        backends.insert(id, BackendEntry { url });
        Self {
            backends,
            order: vec![id],
            ..Self::default()
        }
    }
}

#[derive(serde::Deserialize)]
pub struct RegisterRequest {
    pub ip: String,
    pub port: u16,
}

#[derive(serde::Serialize)]
pub struct RegisterResponse {
    pub id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::BackendRegistry;

    // ── register ──────────────────────────────────────────────────────────────

    #[test]
    fn test_register_returns_unique_ids() {
        let mut reg = BackendRegistry::new();
        let id_a = reg.register("127.0.0.1", 8081);
        let id_b = reg.register("127.0.0.1", 8082);
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_register_same_url_returns_existing_id() {
        let mut reg = BackendRegistry::new();
        let id_first = reg.register("10.0.0.1", 8081);
        let id_second = reg.register("10.0.0.1", 8081);
        assert_eq!(id_first, id_second);
        // Must not create a duplicate entry in all_urls.
        assert_eq!(reg.all_urls().len(), 1);
    }

    #[test]
    fn test_register_builds_correct_url() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 9000);
        assert_eq!(reg.any_url().as_deref(), Some("http://10.0.0.1:9000"));
    }

    // ── any_url ───────────────────────────────────────────────────────────────

    #[test]
    fn test_any_url_empty_returns_none() {
        let reg = BackendRegistry::new();
        assert!(reg.any_url().is_none());
    }

    #[test]
    fn test_any_url_returns_first_registered() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 8081);
        reg.register("10.0.0.2", 8081);
        assert_eq!(reg.any_url().as_deref(), Some("http://10.0.0.1:8081"));
    }

    // ── all_urls ──────────────────────────────────────────────────────────────

    #[test]
    fn test_all_urls_empty_returns_empty_vec() {
        let reg = BackendRegistry::new();
        assert!(reg.all_urls().is_empty());
    }

    #[test]
    fn test_all_urls_preserves_registration_order() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 8081);
        reg.register("10.0.0.2", 8082);
        reg.register("10.0.0.3", 8083);
        assert_eq!(
            reg.all_urls(),
            vec![
                "http://10.0.0.1:8081",
                "http://10.0.0.2:8082",
                "http://10.0.0.3:8083",
            ]
        );
    }

    // ── round_robin_url ───────────────────────────────────────────────────────

    #[test]
    fn test_round_robin_empty_returns_none() {
        let mut reg = BackendRegistry::new();
        assert!(reg.round_robin_url().is_none());
    }

    #[test]
    fn test_round_robin_single_backend_always_returns_same() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 8081);
        assert_eq!(
            reg.round_robin_url().as_deref(),
            Some("http://10.0.0.1:8081")
        );
        assert_eq!(
            reg.round_robin_url().as_deref(),
            Some("http://10.0.0.1:8081")
        );
    }

    #[test]
    fn test_round_robin_alternates_across_two_backends() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 8081);
        reg.register("10.0.0.2", 8082);
        let first = reg.round_robin_url().unwrap();
        let second = reg.round_robin_url().unwrap();
        assert_ne!(
            first, second,
            "consecutive calls should hit different backends"
        );
        // Third call wraps back to the first backend.
        assert_eq!(reg.round_robin_url().unwrap(), first);
    }

    #[test]
    fn test_round_robin_distributes_evenly_across_three_backends() {
        let mut reg = BackendRegistry::new();
        reg.register("10.0.0.1", 8081);
        reg.register("10.0.0.2", 8082);
        reg.register("10.0.0.3", 8083);

        let results: Vec<String> = (0..6).map(|_| reg.round_robin_url().unwrap()).collect();

        let count = |host: &str| results.iter().filter(|u| u.contains(host)).count();
        assert_eq!(count("10.0.0.1"), 2);
        assert_eq!(count("10.0.0.2"), 2);
        assert_eq!(count("10.0.0.3"), 2);
    }

    // ── vm mapping ────────────────────────────────────────────────────────────

    #[test]
    fn test_backend_for_vm_unknown_returns_none() {
        let reg = BackendRegistry::new();
        assert!(reg.backend_for_vm("nonexistent-vm").is_none());
    }

    #[test]
    fn test_register_and_lookup_vm() {
        let mut reg = BackendRegistry::new();
        reg.register_vm("vm-123".to_string(), "http://10.0.0.1:8081".to_string());
        assert_eq!(
            reg.backend_for_vm("vm-123").as_deref(),
            Some("http://10.0.0.1:8081")
        );
    }

    #[test]
    fn test_remove_vm_removes_mapping() {
        let mut reg = BackendRegistry::new();
        reg.register_vm("vm-123".to_string(), "http://10.0.0.1:8081".to_string());
        reg.remove_vm("vm-123");
        assert!(reg.backend_for_vm("vm-123").is_none());
    }

    #[test]
    fn test_remove_vm_nonexistent_is_noop() {
        let mut reg = BackendRegistry::new();
        // Must not panic.
        reg.remove_vm("does-not-exist");
    }

    #[test]
    fn test_all_vm_backends_empty_returns_empty_map() {
        let reg = BackendRegistry::new();
        assert!(reg.all_vm_backends().is_empty());
    }

    #[test]
    fn test_all_vm_backends_returns_all_entries() {
        let mut reg = BackendRegistry::new();
        reg.register_vm("vm-a".to_string(), "http://10.0.0.1:8081".to_string());
        reg.register_vm("vm-b".to_string(), "http://10.0.0.2:8082".to_string());
        let map = reg.all_vm_backends();
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get("vm-a").map(String::as_str),
            Some("http://10.0.0.1:8081")
        );
        assert_eq!(
            map.get("vm-b").map(String::as_str),
            Some("http://10.0.0.2:8082")
        );
    }

    #[test]
    fn test_register_vm_overwrites_existing_mapping() {
        let mut reg = BackendRegistry::new();
        reg.register_vm("vm-123".to_string(), "http://10.0.0.1:8081".to_string());
        reg.register_vm("vm-123".to_string(), "http://10.0.0.2:8082".to_string());
        assert_eq!(
            reg.backend_for_vm("vm-123").as_deref(),
            Some("http://10.0.0.2:8082")
        );
    }

    #[test]
    fn test_vm_mappings_are_independent() {
        let mut reg = BackendRegistry::new();
        reg.register_vm("vm-a".to_string(), "http://10.0.0.1:8081".to_string());
        reg.register_vm("vm-b".to_string(), "http://10.0.0.2:8082".to_string());
        assert_eq!(
            reg.backend_for_vm("vm-a").as_deref(),
            Some("http://10.0.0.1:8081")
        );
        assert_eq!(
            reg.backend_for_vm("vm-b").as_deref(),
            Some("http://10.0.0.2:8082")
        );
    }
}

pub async fn register_handler(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::Json(body): axum::Json<RegisterRequest>,
) -> impl axum::response::IntoResponse {
    let id = state.registry.write().await.register(&body.ip, body.port);
    tracing::info!("Backend registered: {}:{} -> {}", body.ip, body.port, id);
    axum::Json(RegisterResponse { id })
}
