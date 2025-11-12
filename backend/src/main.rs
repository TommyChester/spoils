mod db;
mod models;
mod schema;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;
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
    let existing_product = web::block(move || {
        products::table
            .filter(products::barcode.eq(&barcode))
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

    HttpServer::new(move || {
        let cors = Cors::permissive(); // Configure this properly for production

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors)
            .wrap(actix_web::middleware::Logger::default())
            .service(health)
            .service(hello)
            .service(get_product)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
