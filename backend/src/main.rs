use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    message: String,
}

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
        message: "Spoils API is running".to_string(),
    })
}

#[get("/api/hello")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "message": "Hello from Spoils API!"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    log::info!("Starting Spoils API server on port {}", port);

    HttpServer::new(|| {
        let cors = Cors::permissive(); // Configure this properly for production

        App::new()
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .service(health)
            .service(hello)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
