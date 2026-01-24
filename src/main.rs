mod auth;
mod config;
mod db;
mod error;
mod handlers;
mod models;

use std::net::SocketAddr;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "challenges_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database connected and migrations complete");

    // Build router
    let app = create_router(pool.clone(), config.admin_token.clone());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn create_router(pool: sqlx::PgPool, admin_token: String) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes
    let public_routes = Router::new()
        .route("/challenges", get(handlers::list_challenges))
        .route("/challenges/{id}", get(handlers::get_challenge))
        .route("/challenges/{id}/join", post(handlers::join_challenge))
        .route("/challenges/{id}/leaderboard", get(handlers::get_leaderboard))
        .route("/health", get(handlers::health_check))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            auth::optional_auth,
        ));

    // Authenticated routes
    let auth_routes = Router::new()
        .route("/challenges/{id}/progress", post(handlers::report_progress))
        .route("/challenges/{id}/progress", get(handlers::get_progress))
        .route("/challenges/{id}/leave", delete(handlers::leave_challenge))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            auth::require_auth,
        ));

    // Admin routes
    let admin_routes = Router::new()
        .route("/admin/challenges", post(handlers::create_challenge))
        .route("/admin/challenges/{id}", put(handlers::update_challenge))
        .route("/admin/challenges/{id}", delete(handlers::delete_challenge))
        .layer(middleware::from_fn_with_state(
            admin_token,
            auth::require_admin,
        ));

    Router::new()
        .nest("/v1", public_routes)
        .nest("/v1", auth_routes)
        .nest("/v1", admin_routes)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool)
}
