mod db;
mod jobs;
mod models;
mod schema;
mod workers;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use fang::asynk::async_queue::{AsyncQueue, AsyncQueueable};

use crate::db::DbPool;
use crate::jobs::{FetchProductJob, AnalyzeIngredientsJob, SendNotificationJob};
use crate::models::{NewProduct, OpenFoodFactsResponse, Product};
use crate::schema::products;

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

#[get("/api/products/{barcode}")]
async fn get_product(
    barcode: web::Path<String>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let barcode = barcode.into_inner();

    // Check database first
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database connection failed"
            }));
        }
    };

    // Try to find product in database
    let barcode_clone = barcode.clone();
    let existing_product = web::block(move || {
        products::table
            .filter(products::barcode.eq(&barcode_clone))
            .first::<Product>(&mut conn)
            .optional()
    })
    .await;

    match existing_product {
        Ok(Ok(Some(product))) => {
            log::info!("Product {} found in database", barcode);
            return HttpResponse::Ok().json(product);
        }
        Ok(Ok(None)) => {
            log::info!("Product {} not found in database, querying OpenFoodFacts", barcode);
        }
        Ok(Err(e)) => {
            log::error!("Database query error: {}", e);
        }
        Err(e) => {
            log::error!("Blocking error: {}", e);
        }
    }

    // Query OpenFoodFacts API
    let client = reqwest::Client::new();
    let url = format!("https://world.openfoodfacts.org/api/v2/product/{}", barcode);

    let off_response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(e) => {
            log::error!("Failed to query OpenFoodFacts: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to query OpenFoodFacts API"
            }));
        }
    };

    let off_data: OpenFoodFactsResponse = match off_response.json().await {
        Ok(data) => data,
        Err(e) => {
            log::error!("Failed to parse OpenFoodFacts response: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to parse OpenFoodFacts response"
            }));
        }
    };

    // Check if product was found
    if off_data.status != 1 || off_data.product.is_none() {
        return HttpResponse::NotFound().json(serde_json::json!({
            "error": "Product not found"
        }));
    }

    let product_data = off_data.product.unwrap();

    // Extract key fields
    let product_name = product_data.get("product_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let brands = product_data.get("brands")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let categories = product_data.get("categories")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let quantity = product_data.get("quantity")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let image_url = product_data.get("image_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let nutriscore_grade = product_data.get("nutriscore_grade")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let nova_group = product_data.get("nova_group")
        .and_then(|v| v.as_i64())
        .map(|i| i as i32);

    let ecoscore_grade = product_data.get("ecoscore_grade")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let ingredients_text = product_data.get("ingredients_text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let allergens = product_data.get("allergens")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Store in database
    let new_product = NewProduct {
        barcode: barcode.clone(),
        product_name,
        brands,
        categories,
        quantity,
        image_url,
        nutriscore_grade,
        nova_group,
        ecoscore_grade,
        ingredients_text,
        allergens,
        full_response: product_data.clone(),
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection for insert: {}", e);
            // Still return the product data even if we can't store it
            return HttpResponse::Ok().json(product_data);
        }
    };

    let inserted_product = web::block(move || {
        diesel::insert_into(products::table)
            .values(&new_product)
            .get_result::<Product>(&mut conn)
    })
    .await;

    match inserted_product {
        Ok(Ok(product)) => {
            log::info!("Product {} stored in database", barcode);
            HttpResponse::Ok().json(product)
        }
        Ok(Err(e)) => {
            log::error!("Failed to insert product: {}", e);
            // Still return the product data even if we can't store it
            HttpResponse::Ok().json(product_data)
        }
        Err(e) => {
            log::error!("Blocking error on insert: {}", e);
            HttpResponse::Ok().json(product_data)
        }
    }
}

// Job enqueueing endpoints
#[derive(Deserialize)]
struct EnqueueProductJobRequest {
    barcode: String,
}

#[post("/api/jobs/fetch-product")]
async fn enqueue_fetch_product(
    body: web::Json<EnqueueProductJobRequest>,
) -> impl Responder {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut queue = AsyncQueue::builder()
        .uri(database_url)
        .max_pool_size(3_u32)
        .build();

    match queue.connect().await {
        Ok(_) => {
            let job = FetchProductJob {
                barcode: body.barcode.clone(),
            };

            match queue.insert_task(&job).await {
                Ok(_) => {
                    log::info!("Enqueued fetch product job for barcode: {}", body.barcode);
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Job enqueued successfully",
                        "barcode": body.barcode
                    }))
                }
                Err(e) => {
                    log::error!("Failed to enqueue job: {:?}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to enqueue job"
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to connect to job queue: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to connect to job queue"
            }))
        }
    }
}

#[derive(Deserialize)]
struct EnqueueAnalysisJobRequest {
    product_id: i32,
}

#[post("/api/jobs/analyze-ingredients")]
async fn enqueue_analyze_ingredients(
    body: web::Json<EnqueueAnalysisJobRequest>,
) -> impl Responder {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut queue = AsyncQueue::builder()
        .uri(database_url)
        .max_pool_size(3_u32)
        .build();

    match queue.connect().await {
        Ok(_) => {
            let job = AnalyzeIngredientsJob {
                product_id: body.product_id,
            };

            match queue.insert_task(&job).await {
                Ok(_) => {
                    log::info!("Enqueued ingredient analysis job for product: {}", body.product_id);
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Analysis job enqueued successfully",
                        "product_id": body.product_id
                    }))
                }
                Err(e) => {
                    log::error!("Failed to enqueue analysis job: {:?}", e);
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to enqueue job"
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to connect to job queue: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to connect to job queue"
            }))
        }
    }
}

#[get("/api/jobs/status")]
async fn job_status() -> impl Responder {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut queue = AsyncQueue::builder()
        .uri(database_url)
        .max_pool_size(3_u32)
        .build();

    match queue.connect().await {
        Ok(_) => {
            // Query job statistics
            HttpResponse::Ok().json(serde_json::json!({
                "message": "Job queue is operational",
                "status": "running"
            }))
        }
        Err(e) => {
            log::error!("Failed to connect to job queue: {:?}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to connect to job queue"
            }))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    log::info!("Starting Spoils API server on port {}", port);

    // Initialize database connection pool
    let pool = db::establish_connection_pool();
    log::info!("Database connection pool established");

    // Start background worker pool in a separate task
    tokio::spawn(async move {
        log::info!("Starting background job worker pool...");
        workers::start_worker_pool().await;
    });

    log::info!("Worker pool started in background");

    HttpServer::new(move || {
        let cors = Cors::permissive(); // Configure this properly for production

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .service(health)
            .service(hello)
            .service(get_product)
            .service(enqueue_fetch_product)
            .service(enqueue_analyze_ingredients)
            .service(job_status)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
