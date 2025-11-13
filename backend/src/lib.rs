// Re-export modules for testing
pub mod db;
pub mod jobs;
pub mod models;
pub mod schema;

// Re-export endpoint functions for integration tests
pub use crate::handlers::{health, hello};

mod handlers {
    use actix_web::{get, HttpResponse, Responder};
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct HealthResponse {
        pub status: String,
        pub message: String,
    }

    #[get("/health")]
    pub async fn health() -> impl Responder {
        HttpResponse::Ok().json(HealthResponse {
            status: "ok".to_string(),
            message: "Spoils API is running".to_string(),
        })
    }

    #[get("/api/hello")]
    pub async fn hello() -> impl Responder {
        HttpResponse::Ok().json(serde_json::json!({
            "message": "Hello from Spoils API!"
        }))
    }
}
