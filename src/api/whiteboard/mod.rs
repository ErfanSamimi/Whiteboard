// --- Imports and Type Definitions ---

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
};
use axum::extract::ws::{Message, WebSocket, Utf8Bytes};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use redis::AsyncCommands;
use tokio::time::{timeout, Duration};

// Represents a channel to send messages to a WebSocket client
// Each client connection will have one such sender
type ClientTx = mpsc::UnboundedSender<Message>;

// Groups is a shared, thread-safe map from group names to a list of client senders
// This allows broadcasting messages to all clients in a group
type Groups = Arc<RwLock<HashMap<String, Vec<ClientTx>>>>;

// Application state shared between all handlers
#[derive(Clone)]
struct AppState {
    groups: Groups,
    redis_client: redis::Client, // Used for cross-server pub/sub
}

// --- WebSocket Handler ---

// Called when a new client connects to a WebSocket group
async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(group): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Upgrade HTTP to WebSocket and handle connection
    ws.on_upgrade(move |socket| handle_connection(socket, group, state))
}

// Manages a single WebSocket connection
async fn handle_connection(stream: WebSocket, group: String, state: AppState) {

    let (mut sender_ws, mut receiver_ws) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

     // --- Authenticate within 5 seconds ---
     let token = match timeout(Duration::from_secs(5), receiver_ws.next()).await {
        Ok(Some(Ok(Message::Text(auth_msg)))) => auth_msg,
        _ => {
            let _ = sender_ws.send(Message::Close(None)).await;
            return;
        }
    };

    // Replace with actual token validation logic
    if token != "Bearer supersecrettoken" {
        let _ = sender_ws.send(Message::Close(None)).await;
        return;
    }

    // Register this connection in the group
    {
        let mut group_map = state.groups.write().await;
        group_map.entry(group.clone()).or_default().push(tx.clone());
    }

    // Task: send messages from the channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender_ws.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Clone group and Redis client for the receiving task
    let group_clone = group.clone();
    let redis_client = state.redis_client.clone();

    // Task: receive messages from the WebSocket and publish to Redis
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver_ws.next().await {
            let mut conn = match redis_client.get_async_connection().await {
                Ok(c) => c,
                Err(_) => continue,
            };
            let _ = conn.publish::<_, _, ()>(format!("group:{}", group_clone), text.to_string()).await;
        }
    });

    // Wait for either task to finish (disconnect or error)
    let _ = tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    };

    // Remove this client from the group after disconnect
    let mut group_map = state.groups.write().await;
    if let Some(members) = group_map.get_mut(&group) {
        members.retain(|member| !member.same_channel(&tx));
        if members.is_empty() {
            group_map.remove(&group);
        }
    }
}

// --- Redis Subscriber Task ---

// Listens for messages published to Redis and sends them to local WebSocket clients
async fn redis_subscriber(state: AppState) {
    // Create Redis connection and extract pubsub
    let mut conn = match state.redis_client.get_async_connection().await {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut pubsub = conn.into_pubsub();

    // Subscribe to all group channels
    let _: () = pubsub.psubscribe("group:*").await.unwrap();

    // Continuously listen for new messages
    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name();
        let payload: String = msg.get_payload().unwrap_or_default();

        // Extract group name from channel
        if let Some(group) = channel.strip_prefix("group:") {
            // Send message to all local clients in the group
            let group_map = state.groups.read().await;
            if let Some(members) = group_map.get(group) {
                for member in members {
                    let _ = member.send(Message::Text(Utf8Bytes::from(payload.clone())));
                }
            }
        }
    }
}

// --- Utility Trait ---

// Trait to compare if two mpsc senders refer to the same client connection
trait ChannelEq {
    fn same_channel(&self, other: &Self) -> bool;
}

impl ChannelEq for ClientTx {
    fn same_channel(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}