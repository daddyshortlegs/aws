use axum::{
    body::Body,
    extract::Query,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::IntoResponse,
};
use reqwest::Client;
use std::collections::HashMap;
use tracing::{error, info};

pub struct ProxyService {
    client: Client,
    backend_url: String,
}

impl ProxyService {
    pub fn new(backend_url: String) -> Self {
        let client = Client::new();
        Self {
            client,
            backend_url,
        }
    }

    pub async fn proxy_request(
        &self,
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Option<Body>,
        query: Option<Query<HashMap<String, String>>>,
    ) -> impl IntoResponse {
        let path = uri.path();
        let backend_url = format!("{}{}", self.backend_url, path);

        info!("Proxying {} {} -> {}", method, path, backend_url);

        // Convert Axum Method to Reqwest Method
        let reqwest_method = match method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            "OPTIONS" => reqwest::Method::OPTIONS,
            _ => reqwest::Method::GET, // Default fallback
        };

        // Build the request
        let mut request_builder = self.client.request(reqwest_method, &backend_url);

        // Add headers (excluding host and connection headers)
        for (key, value) in headers.iter() {
            if key != "host" && key != "connection" {
                // Convert Axum header types to Reqwest header types
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_str().as_bytes()),
                    reqwest::header::HeaderValue::from_bytes(value.as_bytes()),
                ) {
                    request_builder = request_builder.header(name, val);
                }
            }
        }

        // Add query parameters if present
        if let Some(query) = query {
            request_builder = request_builder.query(&query.0);
        }

        // Add body for POST/PUT/PATCH requests
        let request = if let Some(body) = body {
            // Convert Axum Body to bytes first
            let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    error!("Failed to read body: {}", e);
                    return (StatusCode::BAD_REQUEST, "Failed to read request body")
                        .into_response();
                }
            };

            // Convert to Reqwest Body
            let reqwest_body = reqwest::Body::from(body_bytes);
            request_builder.body(reqwest_body).build().unwrap()
        } else {
            request_builder.build().unwrap()
        };

        // Execute the request
        match self.client.execute(request).await {
            Ok(response) => {
                let status = response.status();
                let headers = response.headers().clone();
                let body_bytes = response.bytes().await.unwrap_or_default();

                info!("Backend response: {}", status);

                // Convert Reqwest status to Axum status
                let axum_status = StatusCode::from_u16(status.as_u16())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                // Convert response back to axum response
                let mut response_builder = axum::http::Response::builder().status(axum_status);

                // Copy headers from backend response
                for (key, value) in headers.iter() {
                    if key != "transfer-encoding" {
                        // Convert Reqwest header types to Axum header types
                        if let (Ok(name), Ok(val)) = (
                            axum::http::HeaderName::from_bytes(key.as_str().as_bytes()),
                            axum::http::HeaderValue::from_bytes(value.as_bytes()),
                        ) {
                            response_builder = response_builder.header(name, val);
                        }
                    }
                }

                response_builder
                    .body(Body::from(body_bytes))
                    .unwrap()
                    .into_response()
            }
            Err(e) => {
                error!("Proxy request failed: {}", e);
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to proxy request: {}", e),
                )
                    .into_response()
            }
        }
    }
}
