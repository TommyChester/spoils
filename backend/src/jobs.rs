use async_trait::async_trait;
use fang::asynk::async_queue::AsyncQueueable;
use fang::{AsyncRunnable, Deserialize, FangError, Scheduled, Serialize};
use serde_json::Value;

/// Job to fetch and cache a product from OpenFoodFacts
#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct FetchProductJob {
    pub barcode: String,
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for FetchProductJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!("Processing FetchProductJob for barcode: {}", self.barcode);

        // Fetch from OpenFoodFacts API
        let client = reqwest::Client::new();
        let url = format!(
            "https://world.openfoodfacts.org/api/v2/product/{}",
            self.barcode
        );

        match client.get(&url).send().await {
            Ok(response) => match response.json::<Value>().await {
                Ok(_data) => {
                    log::info!("Successfully fetched product {}", self.barcode);
                    // Here you would normally save to database
                    // For now just log success
                    Ok(())
                }
                Err(e) => {
                    log::error!("Failed to parse response for {}: {}", self.barcode, e);
                    Err(FangError {
                        description: format!("Parse error: {}", e),
                    })
                }
            },
            Err(e) => {
                log::error!("Failed to fetch product {}: {}", self.barcode, e);
                Err(FangError {
                    description: format!("Fetch error: {}", e),
                })
            }
        }
    }

    fn uniq(&self) -> bool {
        true
    }

    fn task_type(&self) -> String {
        "fetch_product".to_string()
    }

    fn max_retries(&self) -> i32 {
        3
    }

    fn backoff(&self, attempt: u32) -> u32 {
        // Exponential backoff: 60s, 120s, 240s
        60 * (2_u32.pow(attempt))
    }
}

/// Job to process ingredient analysis
#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct AnalyzeIngredientsJob {
    pub product_id: i32,
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for AnalyzeIngredientsJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!(
            "Processing AnalyzeIngredientsJob for product_id: {}",
            self.product_id
        );

        // Simulate analysis work
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        log::info!("Completed ingredient analysis for {}", self.product_id);
        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn task_type(&self) -> String {
        "analyze_ingredients".to_string()
    }

    fn max_retries(&self) -> i32 {
        2
    }
}

/// Job to send notifications (email, push, etc.)
#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct SendNotificationJob {
    pub user_id: i32,
    pub notification_type: String,
    pub message: String,
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for SendNotificationJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!(
            "Sending {} notification to user {}: {}",
            self.notification_type,
            self.user_id,
            self.message
        );

        // Simulate sending notification
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        log::info!(
            "Successfully sent notification to user {}",
            self.user_id
        );
        Ok(())
    }

    fn uniq(&self) -> bool {
        false // Allow multiple notifications
    }

    fn task_type(&self) -> String {
        "send_notification".to_string()
    }

    fn max_retries(&self) -> i32 {
        5
    }
}

/// Recurring job to clean up old data
#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct CleanupJob {}

/// Job to create a new ingredient
#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct CreateIngredientJob {
    pub name: String,
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for CleanupJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!("Running cleanup job");

        // Simulate cleanup work
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        log::info!("Cleanup completed");
        Ok(())
    }

    fn uniq(&self) -> bool {
        true
    }

    fn task_type(&self) -> String {
        "cleanup".to_string()
    }

    fn cron(&self) -> Option<Scheduled> {
        // Run every day at 2 AM
        Some(Scheduled::CronPattern("0 2 * * *".to_string()))
    }

    fn max_retries(&self) -> i32 {
        1
    }
}

#[typetag::serde]
#[async_trait]
impl AsyncRunnable for CreateIngredientJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!("Creating ingredient: {}", self.name);

        // Fetch nutritional data from USDA FoodData Central
        let usda_data = self.fetch_usda_data().await;

        // Get database URL
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        // Establish database connection
        use diesel::r2d2::{self, ConnectionManager};
        use diesel::{PgConnection, RunQueryDsl};
        use crate::models::NewIngredient;
        use crate::schema::ingredients;

        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = r2d2::Pool::builder()
            .max_size(3)
            .build(manager)
            .expect("Failed to create pool");

        let mut conn = pool.get().expect("Failed to get connection from pool");

        // Create new ingredient with nutritional data if available
        let new_ingredient = if let Some(data) = usda_data {
            log::info!("Found USDA data for ingredient: {}", self.name);
            NewIngredient {
                name: self.name.clone(),
                branded: false,
                gram_protein_per_gram: data.protein,
                gram_carbs_per_gram: data.carbs,
                gram_fat_per_gram: data.fat,
                gram_fiber_per_gram: data.fiber,
            }
        } else {
            log::info!("No USDA data found, creating ingredient with name only: {}", self.name);
            NewIngredient {
                name: self.name.clone(),
                branded: false,
                gram_protein_per_gram: None,
                gram_carbs_per_gram: None,
                gram_fat_per_gram: None,
                gram_fiber_per_gram: None,
            }
        };

        match diesel::insert_into(ingredients::table)
            .values(&new_ingredient)
            .execute(&mut conn)
        {
            Ok(_) => {
                log::info!("Successfully created ingredient: {}", self.name);
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to create ingredient '{}': {}", self.name, e);
                Err(FangError {
                    description: format!("Database error: {}", e),
                })
            }
        }
    }

    fn uniq(&self) -> bool {
        true // Prevent duplicate creation jobs for the same ingredient
    }

    fn task_type(&self) -> String {
        "create_ingredient".to_string()
    }

    fn max_retries(&self) -> i32 {
        3
    }
}

#[derive(Debug)]
struct USDANutritionData {
    protein: Option<f32>,
    carbs: Option<f32>,
    fat: Option<f32>,
    fiber: Option<f32>,
}

impl CreateIngredientJob {
    /// Fetch nutritional data from USDA FoodData Central API
    async fn fetch_usda_data(&self) -> Option<USDANutritionData> {
        // Get API key from environment (optional - has demo key fallback)
        let api_key = std::env::var("USDA_API_KEY")
            .unwrap_or_else(|_| "DEMO_KEY".to_string());

        let client = reqwest::Client::new();
        let url = format!(
            "https://api.nal.usda.gov/fdc/v1/foods/search?api_key={}&query={}",
            api_key,
            urlencoding::encode(&self.name)
        );

        log::info!("Searching USDA FoodData Central for: {}", self.name);

        match client.get(&url).send().await {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        // Check if we got any foods back
                        let foods = data.get("foods").and_then(|f| f.as_array());

                        if let Some(foods_array) = foods {
                            if let Some(first_food) = foods_array.first() {
                                log::info!("Found USDA match for '{}': {}",
                                    self.name,
                                    first_food.get("description")
                                        .and_then(|d| d.as_str())
                                        .unwrap_or("unknown")
                                );

                                return self.extract_nutrition_data(first_food);
                            }
                        }

                        log::info!("No USDA results found for: {}", self.name);
                        None
                    }
                    Err(e) => {
                        log::error!("Failed to parse USDA response for '{}': {}", self.name, e);
                        None
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to fetch USDA data for '{}': {}", self.name, e);
                None
            }
        }
    }

    /// Extract nutrition data from USDA food item
    fn extract_nutrition_data(&self, food: &serde_json::Value) -> Option<USDANutritionData> {
        let nutrients = food.get("foodNutrients").and_then(|n| n.as_array())?;

        let mut protein = None;
        let mut carbs = None;
        let mut fat = None;
        let mut fiber = None;

        // USDA nutrient IDs (from FoodData Central)
        // 1003 = Protein, 1005 = Carbs, 1004 = Fat, 1079 = Fiber
        for nutrient in nutrients {
            if let Some(nutrient_id) = nutrient.get("nutrientId").and_then(|id| id.as_i64()) {
                if let Some(value) = nutrient.get("value").and_then(|v| v.as_f64()) {
                    // Convert from per 100g to per 1g
                    let value_per_gram = (value / 100.0) as f32;

                    match nutrient_id {
                        1003 => protein = Some(value_per_gram), // Protein
                        1005 => carbs = Some(value_per_gram),   // Carbs
                        1004 => fat = Some(value_per_gram),     // Fat
                        1079 => fiber = Some(value_per_gram),   // Fiber
                        _ => {}
                    }
                }
            }
        }

        log::info!(
            "Extracted nutrition for '{}': protein={:?}g, carbs={:?}g, fat={:?}g, fiber={:?}g per gram",
            self.name, protein, carbs, fat, fiber
        );

        Some(USDANutritionData {
            protein,
            carbs,
            fat,
            fiber,
        })
    }
}
