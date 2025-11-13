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
use fang::NoTls;

use crate::db::DbPool;
use crate::jobs::{FetchProductJob, AnalyzeIngredientsJob};
use crate::models::{NewProduct, OpenFoodFactsResponse, Product, Ingredient, ProductNonFood, NewProductNonFood};
use crate::schema::{products, products_non_food};

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

            // Process ingredients - extract and enqueue for creation if needed
            process_product_ingredients(&product_data, &pool);

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

/// Process ingredients from product data and enqueue for creation if needed
fn process_product_ingredients(product_data: &serde_json::Value, pool: &web::Data<DbPool>) {
    // Try to get ingredients array from OpenFoodFacts data
    let ingredients_array = product_data
        .get("ingredients")
        .and_then(|v| v.as_array());

    if let Some(ingredients) = ingredients_array {
        log::info!("Processing {} ingredients from product", ingredients.len());

        // Get a database connection
        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Failed to get DB connection for ingredient processing: {}", e);
                return;
            }
        };

        // Process each ingredient
        for ingredient in ingredients {
            // Extract ingredient name (can be "text", "id", or other fields)
            let ingredient_name = ingredient
                .get("text")
                .or_else(|| ingredient.get("id"))
                .and_then(|v| v.as_str());

            if let Some(name) = ingredient_name {
                // Clean up the ingredient name
                let clean_name = name.trim();

                if !clean_name.is_empty() {
                    log::info!("Processing ingredient: {}", clean_name);

                    // Find or enqueue for creation
                    match Ingredient::find_or_enqueue_for_creation(clean_name, &mut conn) {
                        Ok(Some(id)) => {
                            log::info!("Ingredient '{}' found with ID: {}", clean_name, id);
                        }
                        Ok(None) => {
                            log::info!("Ingredient '{}' enqueued for creation", clean_name);
                        }
                        Err(e) => {
                            log::error!("Error processing ingredient '{}': {}", clean_name, e);
                        }
                    }
                }
            }
        }
    } else {
        // Fallback: try to parse ingredients_text (comma-separated string)
        if let Some(ingredients_text) = product_data
            .get("ingredients_text")
            .and_then(|v| v.as_str())
        {
            log::info!("Processing ingredients from text: {}", ingredients_text);

            let mut conn = match pool.get() {
                Ok(conn) => conn,
                Err(e) => {
                    log::error!("Failed to get DB connection for ingredient processing: {}", e);
                    return;
                }
            };

            // Split by commas and process each ingredient
            for ingredient_name in ingredients_text.split(',') {
                let clean_name = ingredient_name.trim();

                if !clean_name.is_empty() {
                    log::info!("Processing ingredient: {}", clean_name);

                    match Ingredient::find_or_enqueue_for_creation(clean_name, &mut conn) {
                        Ok(Some(id)) => {
                            log::info!("Ingredient '{}' found with ID: {}", clean_name, id);
                        }
                        Ok(None) => {
                            log::info!("Ingredient '{}' enqueued for creation", clean_name);
                        }
                        Err(e) => {
                            log::error!("Error processing ingredient '{}': {}", clean_name, e);
                        }
                    }
                }
            }
        } else {
            log::info!("No ingredients data found in product");
        }
    }
}

/// Process ingredients from non-food products (supplements, beauty, etc.)
fn process_non_food_ingredients(product: &ProductNonFood, pool: &web::Data<DbPool>) {
    log::info!("Extracting ingredients from non-food product: {}", product.name);

    // Try to extract ingredients from description
    // Look for patterns like "Ingredients:" or "Contains:" followed by comma-separated list
    let ingredients_text = if let Some(ref description) = product.description {
        extract_ingredients_from_text(description)
    } else {
        None
    };

    if let Some(ingredients) = ingredients_text {
        log::info!("Found ingredients in description: {}", ingredients);

        let mut conn = match pool.get() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Failed to get DB connection for ingredient processing: {}", e);
                return;
            }
        };

        // Collect ingredient names
        let ingredient_names: Vec<String> = ingredients
            .split(',')
            .map(|name| name.trim().trim_end_matches('.').trim_end_matches(';').to_string())
            .filter(|name| {
                !name.is_empty() &&
                name.len() >= 2 &&
                !name.eq_ignore_ascii_case("and") &&
                !name.eq_ignore_ascii_case("or")
            })
            .collect();

        if ingredient_names.is_empty() {
            log::info!("No valid ingredients found after filtering");
            return;
        }

        log::info!("Processing {} ingredients", ingredient_names.len());

        // Spawn async task to enqueue all ingredients sequentially with single queue connection
        tokio::spawn(async move {
            use fang::asynk::async_queue::{AsyncQueue, AsyncQueueable};
            use fang::NoTls;
            use crate::jobs::CreateIngredientJob;

            let database_url = match std::env::var("DATABASE_URL") {
                Ok(url) => url,
                Err(_) => {
                    log::error!("DATABASE_URL not set");
                    return;
                }
            };

            let mut queue = AsyncQueue::builder()
                .uri(database_url)
                .max_pool_size(2_u32)
                .build();

            // Connect once and reuse the connection
            let connect_result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                queue.connect(NoTls)
            ).await;

            match connect_result {
                Ok(Ok(_)) => {
                    log::info!("Connected to job queue for ingredient processing");

                    // Process ingredients sequentially to avoid overwhelming the connection pool
                    for ingredient_name in ingredient_names {
                        let job = CreateIngredientJob {
                            name: ingredient_name.clone(),
                        };

                        match queue.insert_task(&job).await {
                            Ok(_) => {
                                log::info!("Successfully enqueued CreateIngredientJob for '{}'", ingredient_name);
                            }
                            Err(e) => {
                                log::error!("Failed to enqueue job for '{}': {:?}", ingredient_name, e);
                            }
                        }

                        // Small delay between insertions to avoid rate limiting
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    }

                    log::info!("Finished enqueueing all ingredient jobs");
                }
                Ok(Err(e)) => {
                    log::error!("Failed to connect to job queue: {:?}", e);
                }
                Err(_) => {
                    log::error!("Timeout connecting to job queue");
                }
            }
        });

        // Mark ingredients as found or enqueued in the sync code
        for ingredient_name in ingredients.split(',') {
            let clean_name = ingredient_name
                .trim()
                .trim_end_matches('.')
                .trim_end_matches(';');

            if clean_name.is_empty() ||
               clean_name.len() < 2 ||
               clean_name.eq_ignore_ascii_case("and") ||
               clean_name.eq_ignore_ascii_case("or") {
                continue;
            }

            log::info!("Processing ingredient: {}", clean_name);

            match Ingredient::find_in_db(clean_name, &mut conn) {
                Ok(Some(id)) => {
                    log::info!("Ingredient '{}' found with ID: {}", clean_name, id);
                }
                Ok(None) => {
                    log::info!("Ingredient '{}' enqueued for creation", clean_name);
                }
                Err(e) => {
                    log::error!("Error checking ingredient '{}': {}", clean_name, e);
                }
            }
        }
    } else {
        log::info!("No ingredients found in product description");
    }
}

/// Extract ingredients from text by looking for "Ingredients:", "Contains:", etc.
fn extract_ingredients_from_text(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();

    // Look for common ingredient markers
    let markers = [
        "ingredients:",
        "contains:",
        "active ingredients:",
        "inactive ingredients:",
        "other ingredients:",
    ];

    for marker in &markers {
        if let Some(start_idx) = text_lower.find(marker) {
            let ingredients_start = start_idx + marker.len();
            let remaining_text = &text[ingredients_start..];

            // Take until we hit a period followed by capital letter, or end of string
            // This helps separate the ingredient list from following sentences
            let mut end_idx = remaining_text.len();

            // Look for common ending patterns
            if let Some(idx) = remaining_text.find(". ") {
                // Check if next character is uppercase (likely new sentence)
                if let Some(next_char) = remaining_text.chars().nth(idx + 2) {
                    if next_char.is_uppercase() {
                        end_idx = idx;
                    }
                }
            }

            let ingredients = remaining_text[..end_idx].trim();
            if !ingredients.is_empty() {
                return Some(ingredients.to_string());
            }
        }
    }

    None
}

// ============= Non-Food Products Endpoints =============

#[get("/api/products-non-food/{barcode}")]
async fn get_product_non_food(
    barcode: web::Path<String>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let barcode = barcode.into_inner();

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
        products_non_food::table
            .filter(products_non_food::barcode.eq(&barcode_clone))
            .first::<ProductNonFood>(&mut conn)
            .optional()
    })
    .await;

    match existing_product {
        Ok(Ok(Some(product))) => {
            log::info!("Non-food product {} found in database", barcode);
            HttpResponse::Ok().json(product)
        }
        Ok(Ok(None)) => {
            log::info!("Non-food product {} not found in database", barcode);
            HttpResponse::NotFound().json(serde_json::json!({
                "error": "Product not found",
                "barcode": barcode
            }))
        }
        Ok(Err(e)) => {
            log::error!("Database query error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database query failed"
            }))
        }
        Err(e) => {
            log::error!("Blocking error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

#[derive(Deserialize)]
struct CreateProductNonFoodRequest {
    barcode: Option<String>,
    name: String,
    brand: Option<String>,
    category: Option<String>,
    description: Option<String>,
    data_source: Option<String>,
}

#[post("/api/products-non-food")]
async fn create_product_non_food(
    body: web::Json<CreateProductNonFoodRequest>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let new_product = NewProductNonFood {
        barcode: body.barcode.clone(),
        name: body.name.clone(),
        brand: body.brand.clone(),
        category: body.category.clone(),
        description: body.description.clone(),
        full_response: None,
        data_source: body.data_source.clone(),
    };

    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database connection failed"
            }));
        }
    };

    let inserted_product = web::block(move || {
        diesel::insert_into(products_non_food::table)
            .values(&new_product)
            .get_result::<ProductNonFood>(&mut conn)
    })
    .await;

    match inserted_product {
        Ok(Ok(product)) => {
            log::info!("Non-food product '{}' created with ID: {}", product.name, product.id);

            // Process ingredients for supplements and beauty products
            if let Some(ref category) = product.category {
                let category_lower = category.to_lowercase();
                if category_lower.contains("supplement") ||
                   category_lower.contains("beauty") ||
                   category_lower.contains("cosmetic") ||
                   category_lower.contains("skincare") ||
                   category_lower.contains("vitamin") {
                    log::info!("Processing ingredients for {} product: {}", category, product.name);
                    process_non_food_ingredients(&product, &pool);
                }
            }

            HttpResponse::Created().json(product)
        }
        Ok(Err(e)) => {
            log::error!("Failed to create non-food product: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create product",
                "details": format!("{}", e)
            }))
        }
        Err(e) => {
            log::error!("Blocking error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
        }
    }
}

#[get("/api/products-non-food")]
async fn list_products_non_food(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database connection failed"
            }));
        }
    };

    let products = web::block(move || {
        products_non_food::table
            .order(products_non_food::created_at.desc())
            .limit(100)
            .load::<ProductNonFood>(&mut conn)
    })
    .await;

    match products {
        Ok(Ok(products_list)) => {
            log::info!("Retrieved {} non-food products", products_list.len());
            HttpResponse::Ok().json(serde_json::json!({
                "products": products_list,
                "count": products_list.len()
            }))
        }
        Ok(Err(e)) => {
            log::error!("Database query error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Database query failed"
            }))
        }
        Err(e) => {
            log::error!("Blocking error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Internal server error"
            }))
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

    match queue.connect(NoTls).await {
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

    match queue.connect(NoTls).await {
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

    match queue.connect(NoTls).await {
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
            .service(get_product_non_food)
            .service(create_product_non_food)
            .service(list_products_non_food)
            .service(enqueue_fetch_product)
            .service(enqueue_analyze_ingredients)
            .service(job_status)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_ingredients_with_ingredients_marker() {
        let text = "Premium supplement. Ingredients: Vitamin C, Zinc, Magnesium. Take daily.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("Vitamin C"));
        assert!(ingredients.contains("Zinc"));
        assert!(ingredients.contains("Magnesium"));
        assert!(!ingredients.contains("Take daily")); // Should stop at period before capital
    }

    #[test]
    fn test_extract_ingredients_with_contains_marker() {
        let text = "Natural formula. Contains: Water, Glycerin, Hyaluronic Acid.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("Water"));
        assert!(ingredients.contains("Glycerin"));
        assert!(ingredients.contains("Hyaluronic Acid"));
    }

    #[test]
    fn test_extract_ingredients_with_active_ingredients() {
        let text = "Active Ingredients: Retinol, Niacinamide, Peptides. For external use only.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("Retinol"));
        assert!(ingredients.contains("Niacinamide"));
    }

    #[test]
    fn test_extract_ingredients_no_marker() {
        let text = "This is a product with no ingredient list in it.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_ingredients_multiple_sentences() {
        let text = "Product description. Ingredients: Salt, Pepper, Garlic. Directions: Use as needed. Storage: Keep cool.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("Salt"));
        assert!(ingredients.contains("Garlic"));
        // Should stop before "Directions" (capital letter after period)
        assert!(!ingredients.contains("Directions"));
    }

    #[test]
    fn test_extract_ingredients_case_insensitive() {
        let text = "INGREDIENTS: WATER, SUGAR, SALT";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("WATER"));
        assert!(ingredients.contains("SUGAR"));
    }

    #[test]
    fn test_extract_ingredients_with_other_ingredients_marker() {
        let text = "Supplement facts. Other Ingredients: Cellulose, Silica. Made in USA.";
        let result = extract_ingredients_from_text(text);

        assert!(result.is_some());
        let ingredients = result.unwrap();
        assert!(ingredients.contains("Cellulose"));
        assert!(ingredients.contains("Silica"));
    }
}
