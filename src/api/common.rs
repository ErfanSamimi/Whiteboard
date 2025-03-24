use sqlx::postgres::PgPool;
use axum::{
    extract::{FromRequestParts, FromRef},
    http::{request::Parts, StatusCode},
};
use std::sync::Arc;
use tokio::sync::Mutex;
use redis::Client as RedisClient;
use mongodb::Client as MongoClient;

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: Arc<PgPool>,
    pub redis_client: Arc<RedisClient>,
    pub mongo_client: Arc<MongoClient>,
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