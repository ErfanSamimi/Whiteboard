mod whiteboard;
mod user;
mod api;
mod project;

extern crate dotenv;


use axum::{
    routing::{post, get},
    Router,
};
use std::net::SocketAddr;
use http::Method;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use tower_http::cors::{CorsLayer, Any};
use dotenv::dotenv;
use tower::ServiceBuilder;
use mongodb::Client as MongoClient;
use std::error::Error;
use tracing::{error, info, instrument};
use std::sync::Arc;
use redis::Client as RedisClient;
use api::common::AppState;



#[instrument(skip(pg_pool), name = "postgres_health_check")]
async fn postgres_health_check(pg_pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(pg_pool).await?;
    Ok(())
}

#[instrument(skip(redis_client), name = "redis_health_check")]
async fn redis_health_check(redis_client: &RedisClient) -> Result<(), redis::RedisError> {
    let mut conn = redis_client.get_connection()?;
    let _: String = redis::cmd("PING").query(&mut conn)?;
    Ok(())
}

#[instrument(skip(mongo_client), name = "mongo_health_check")]
async fn mongo_health_check(mongo_client: &MongoClient) -> Result<(), mongodb::error::Error> {
    mongo_client.list_database_names().await?;
    Ok(())
}

#[instrument(name = "create_app_state")]
async fn create_app_state() -> Result<AppState, Box<dyn Error>> {
    let pg_conn_string = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL environment variable not set")?;
    let redis_conn_string = env::var("REDIS_CONNECTION_STRING")
        .map_err(|_| "REDIS_CONNECTION_STRING environment variable not set")?;
    let mongo_conn_string = env::var("MONGO_CONNECTION_STRING")
        .map_err(|_| "MONGO_CONNECTION_STRING environment variable not set")?;

    let pg_pool = PgPoolOptions::new()
        .max_connections(5) // Adjust as needed
        .connect(&pg_conn_string)
        .await?;

    if let Err(e) = postgres_health_check(&pg_pool).await {
        error!("Postgres health check failed: {}", e);
        return Err(Box::new(e));
    }
    info!("Postgres connection pool established.");

    let redis_client = redis::Client::open(redis_conn_string)?;
    if let Err(e) = redis_health_check(&redis_client).await {
        error!("Redis health check failed: {}", e);
        return Err(Box::new(e));
    }
    let redis_client = Arc::new(redis_client);
    info!("Redis connection established.");

    let mongo_client = MongoClient::with_uri_str(&mongo_conn_string).await?;
    if let Err(e) = mongo_health_check(&mongo_client).await {
        error!("Mongo health check failed: {}", e);
        return Err(Box::new(e));
    }
    let mongo_client = Arc::new(mongo_client);
    info!("Mongo connection established.");

    Ok(AppState {
        pg_pool: Arc::new(pg_pool),
        redis_client,
        mongo_client,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();


    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer())
        .init();

    let app_state = create_app_state().await?;
    


    let cors_layer = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST])
    .allow_headers(Any)
    .allow_origin(Any);



    let app = Router::new()
        .route("/api/auth/login/", post(api::auth::authorize))
        .route("/api/projects/users/", get(api::user::user_list_view))
        .route("/api/projects/",
             post(api::project::project_creation_view)
            .get(api::project::owned_project_list_view)
            )

        .route("/api/projects/{project_id}/update_collaborators/", post(api::project::add_collaborator_view))
        .route("/api/projects/{project_id}/drawing/", get(api::project::get_whiteboard_data_view))
        .layer(ServiceBuilder::new().layer(cors_layer))
        .with_state(app_state);

        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        println!("listening on {}", addr);
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();

        return Ok(());

}
