use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

pub type WebSocketConnections = Arc<RwLock<HashMap<String, WebSocket>>>;

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    
    info!("WebSocket connection established");

    // Send welcome message
    if let Err(e) = sender.send(Message::Text("Connected to VM Orchestrator WebSocket".to_string())).await {
        error!("Failed to send welcome message: {}", e);
        return;
    }

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("Received text message: {}", text);
                
                // Echo the message back for now
                if let Err(e) = sender.send(Message::Text(format!("Echo: {}", text))).await {
                    error!("Failed to send echo message: {}", e);
                    break;
                }
            }
            Ok(Message::Binary(data)) => {
                info!("Received binary message of {} bytes", data.len());
                
                // Echo binary data back
                if let Err(e) = sender.send(Message::Binary(data)).await {
                    error!("Failed to send binary echo: {}", e);
                    break;
                }
            }
            Ok(Message::Ping(data)) => {
                info!("Received ping");
                if let Err(e) = sender.send(Message::Pong(data)).await {
                    error!("Failed to send pong: {}", e);
                    break;
                }
            }
            Ok(Message::Pong(_)) => {
                info!("Received pong");
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed by client");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection ended");
} 