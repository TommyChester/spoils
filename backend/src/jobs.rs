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

        // Create new ingredient with just the name
        let new_ingredient = NewIngredient {
            name: self.name.clone(),
            branded: false, // Default to non-branded
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
