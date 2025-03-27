use sqlx::postgres::PgPool;
use axum::{
    extract::{FromRequestParts, FromRef},
    http::{request::Parts, StatusCode},
};
use std::{collections::HashMap, sync::Arc};
use redis::Client as RedisClient;
use mongodb::Client as MongoClient;
use tokio::sync::{mpsc, RwLock};
use axum::extract::ws::Message;

// Represents a channel to send messages to a WebSocket client
// Each client connection will have one such sender
pub type ClientTx = mpsc::UnboundedSender<Message>;

// Groups is a shared, thread-safe map from group names to a list of client senders
// This allows broadcasting messages to all clients in a group
pub type Groups = Arc<RwLock<HashMap<i64, Vec<ClientTx>>>>;

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: Arc<PgPool>,
    pub redis_client: Arc<RedisClient>,
    pub mongo_client: Arc<MongoClient>,
    pub ws_groups: Groups,

}


struct DatabaseConnection(sqlx::pool::PoolConnection<sqlx::Postgres>);


impl<S> FromRequestParts<S> for DatabaseConnection
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let pool = PgPool::from_ref(state);

        let conn = pool.acquire().await.map_err(internal_error)?;

        Ok(Self(conn))
    }
}


/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}