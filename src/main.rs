mod whiteboard;
mod user;
mod api;
mod project;

extern crate dotenv;


use axum::{
    routing::{post, get},
    Router,
};

use http::Method;
use sqlx::postgres::PgPoolOptions;
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tower_http::cors::{CorsLayer, Any};
use std::time::Duration;
use dotenv::dotenv;
use tower::ServiceBuilder;



#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // set up connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(env::var("DATABASE_URL").expect("DATABASE_URL must be set").as_str())
        .await
        .expect("can't connect to database");

    let state = api::common::AppState { pool };


    let cors_layer = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST])
    .allow_headers(Any)
    .allow_origin(Any);



    let app = Router::new()
        .route("/api/auth/login/", post(api::auth::authorize).with_state(state.clone()))
        .route("/api/projects/users/", get(api::user::user_list_view).with_state(state.clone()))
        .route("/api/projects/", post(api::project::project_creation_view)
            .get(api::project::owned_project_list_view)
            .with_state(state.clone()))
        .layer(ServiceBuilder::new().layer(cors_layer));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
