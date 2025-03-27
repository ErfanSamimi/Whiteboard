// --- Load required modules ---
mod auth;
mod common;
// --- Imports and Type Definitions ---

use axum::{ extract::{ Path, State, WebSocketUpgrade }, response::IntoResponse, body::Bytes };
use axum::extract::ws::{ Message, WebSocket, Utf8Bytes };
use common::{ compress_data, decompress_data, WsEventReceive, WsEventSend };
use futures::{ stream::SplitSink, SinkExt, StreamExt };
use tokio::sync::mpsc;
use redis::AsyncCommands;
use tokio::time::{ timeout, Duration };

use crate::api::common::{ AppState, ClientTx };

use super::project;

// --- WebSocket Handler ---

// Called when a new client connects to a WebSocket group
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(project_id): Path<i64>,
    State(state): State<AppState>
) -> impl IntoResponse {
    // Upgrade HTTP to WebSocket and handle connection
    let ws_auth_users = auth::WSAuthenticatedUsers::new(
        format!("whiteboard_{}", project_id).as_str(),
        state.redis_client.clone()
    );

    ws.on_upgrade(move |socket| handle_connection(socket, project_id, state, ws_auth_users))
}

async fn send_event_to_ws(
    sender_ws: &mut SplitSink<WebSocket, Message>,
    event: WsEventSend
) -> futures::sink::Send<'_, SplitSink<WebSocket, Message>, Message> {
    let msg_txt = serde_json::to_string(&event).unwrap();
    return sender_ws.send(Message::Text(Utf8Bytes::from(msg_txt)));
}

// Manages a single WebSocket connection
async fn handle_connection(
    stream: WebSocket,
    project_id: i64,
    state: AppState,
    mut ws_auth_users: auth::WSAuthenticatedUsers
) {
    let (mut sender_ws, mut receiver_ws) = stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // --- Authenticate within 5 seconds ---
    let token_event_str = match timeout(Duration::from_secs(5), receiver_ws.next()).await {
        Ok(Some(Ok(Message::Text(auth_msg)))) => auth_msg.to_string(),
        _ => {
            let _ = sender_ws.send(Message::Close(None)).await;
            return;
        }
    };

    // Replace with actual token validation logic
    let token = serde_json::from_str::<WsEventReceive>(&token_event_str);
    match token {
        Ok(event) => {
            let auth_result = auth::authorize(project_id, &state, &event, &mut ws_auth_users).await;
            match &auth_result {
                WsEventSend::AuthSuccess { message, user_token } => {
                    if send_event_to_ws(&mut sender_ws, auth_result).await.await.is_err() {
                        return;
                    }
                }
                _ => {
                    send_event_to_ws(&mut sender_ws, auth_result).await;
                    return;
                }
            }
        }
        Err(_) => {
            let _ = sender_ws.send(Message::Close(None)).await;
            return;
        }
    }

    // Register this connection in the group
    {
        let mut group_map = state.ws_groups.write().await;
        group_map.entry(project_id.clone()).or_default().push(tx.clone());
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
    let group_clone = project_id.clone();
    let redis_client = state.redis_client.clone();

    // Task: receive messages from the WebSocket and publish to Redis
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Binary(comressed_message))) = receiver_ws.next().await {
            let mut conn = match redis_client.get_multiplexed_async_connection().await {
                Ok(c) => c,
                Err(_) => {
                    continue;
                }
            };

            let text = decompress_data(comressed_message.into_iter().collect()).unwrap();

            let event = match serde_json::from_str::<WsEventReceive>(text.as_str()) {
                Ok(e) => WsEventSend::from(&e),
                Err(_) => WsEventSend::Error { message: "invalid message.".to_string() },
            };

            println!("Message {} published", event.get_name());            

            let _ = conn.publish::<_, _, ()>(
                format!("group:{}", group_clone),
                compress_data(serde_json::to_string(&event).unwrap())
            ).await;
        }
    });

    // Wait for either task to finish (disconnect or error)
    let _ = tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    };

    // Remove this client from the group after disconnect
    let mut group_map = state.ws_groups.write().await;
    if let Some(members) = group_map.get_mut(&project_id) {
        members.retain(|member| !member.same_channel(&tx));
        if members.is_empty() {
            group_map.remove(&project_id);
        }
    }
}

// --- Redis Subscriber Task ---

// Listens for messages published to Redis and sends them to local WebSocket clients
pub async fn redis_subscriber(state: AppState) {
    // Create Redis connection and extract pubsub
    let mut conn = match state.redis_client.get_async_connection().await {
        Ok(c) => c,
        Err(_) => {
            return;
        }
    };
    let mut pubsub = conn.into_pubsub();

    // Subscribe to all group channels
    let _: () = pubsub.psubscribe("group:*").await.unwrap();

    // Continuously listen for new messages
    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name();
        let payload: Vec<u8> = msg.get_payload().unwrap_or_default();

        // Extract group name from channel
        if let Some(group) = channel.strip_prefix("group:") {
            let project_id: i64 = group.parse().expect("Invalid number");
            // Send message to all local clients in the group
            let group_map = state.ws_groups.read().await;
            if let Some(members) = group_map.get(&project_id) {
                for member in members {
                    let _ = member.send(Message::Binary(Bytes::from(payload.clone())));
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
