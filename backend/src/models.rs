use diesel::prelude::*;
use diesel::sql_types::Varchar;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Queryable, Serialize, Selectable)]
#[diesel(table_name = crate::schema::products)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Product {
    pub id: i32,
    pub barcode: String,
    pub product_name: Option<String>,
    pub brands: Option<String>,
    pub categories: Option<String>,
    pub quantity: Option<String>,
    pub image_url: Option<String>,
    pub nutriscore_grade: Option<String>,
    pub nova_group: Option<i32>,
    pub ecoscore_grade: Option<String>,
    pub ingredients_text: Option<String>,
    pub allergens: Option<String>,
    pub full_response: serde_json::Value,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct {
    pub barcode: String,
    pub product_name: Option<String>,
    pub brands: Option<String>,
    pub categories: Option<String>,
    pub quantity: Option<String>,
    pub image_url: Option<String>,
    pub nutriscore_grade: Option<String>,
    pub nova_group: Option<i32>,
    pub ecoscore_grade: Option<String>,
    pub ingredients_text: Option<String>,
    pub allergens: Option<String>,
    pub full_response: serde_json::Value,
}

#[derive(Deserialize)]
pub struct OpenFoodFactsResponse {
    pub status: i32,
    pub code: Option<String>,
    pub product: Option<serde_json::Value>,
}

#[derive(Queryable, Serialize, Selectable, Debug)]
#[diesel(table_name = crate::schema::ingredients)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Ingredient {
    pub id: i32,
    pub name: String,
    pub branded: bool,
    pub sub_ingredients: Vec<i32>,
    pub parent_ingredients: Vec<i32>,
    pub gram_protein_per_gram: Option<f32>,
    pub gram_carbs_per_gram: Option<f32>,
    pub gram_fat_per_gram: Option<f32>,
    pub gram_fiber_per_gram: Option<f32>,
    pub vitamins: Option<serde_json::Value>,
    pub minerals: Option<serde_json::Value>,
    pub essential_fatty_acids: Option<serde_json::Value>,
    pub essential_amino_acids: Option<serde_json::Value>,
    pub heavy_metals: Option<serde_json::Value>,
    pub micro_plastics: Option<serde_json::Value>,
    pub industrial_chemicals: Option<serde_json::Value>,
    pub pesticides: Option<serde_json::Value>,
    pub hormones: Option<serde_json::Value>,
    pub antibiotics: Option<serde_json::Value>,
    pub beta_agonists: Option<serde_json::Value>,
    pub antiparasitics: Option<serde_json::Value>,
    pub carcinogens: Option<serde_json::Value>,
    pub natural_toxins: Option<serde_json::Value>,
    pub radiological: Option<serde_json::Value>,
    pub historical_issues: Option<serde_json::Value>,
    pub fraudulent_ingredients: Option<serde_json::Value>,
    pub dyes: Option<serde_json::Value>,
    pub emulsifiers: Option<serde_json::Value>,
    pub preservatives: Option<serde_json::Value>,
    pub gram_trans_fat_per_gram: Option<f32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::ingredients)]
pub struct NewIngredient {
    pub name: String,
    pub branded: bool,
}

impl Ingredient {
    /// Find ingredient by name (case-insensitive) or enqueue job to create it
    /// Returns Option<i32> - ingredient ID if found, None if enqueued for creation
    pub fn find_or_enqueue_for_creation(
        ingredient_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Option<i32>, diesel::result::Error> {
        use crate::schema::ingredients::dsl::*;
        use diesel::dsl::sql;
        use diesel::sql_types::Bool;

        // Try to find with case-insensitive search using ILIKE
        let found = ingredients
            .filter(sql::<Bool>(&format!("LOWER(name) = LOWER('{}')", ingredient_name.replace("'", "''"))))
            .select(id)
            .first::<i32>(conn)
            .optional()?;

        if let Some(ingredient_id) = found {
            log::info!("Found existing ingredient: {} (ID: {})", ingredient_name, ingredient_id);
            return Ok(Some(ingredient_id));
        }

        // Not found - enqueue job to create it
        log::info!("Ingredient '{}' not found, enqueueing creation job", ingredient_name);

        // Import job queue dependencies
        use fang::asynk::async_queue::{AsyncQueue, AsyncQueueable};
        use fang::NoTls;
        use crate::jobs::CreateIngredientJob;

        // Create async runtime for job enqueueing
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

            let mut queue = AsyncQueue::builder()
                .uri(database_url)
                .max_pool_size(3_u32)
                .build();

            match queue.connect(NoTls).await {
                Ok(_) => {
                    let job = CreateIngredientJob {
                        name: ingredient_name.to_string(),
                    };

                    match queue.insert_task(&job).await {
                        Ok(_) => {
                            log::info!("Successfully enqueued CreateIngredientJob for '{}'", ingredient_name);
                        }
                        Err(e) => {
                            log::error!("Failed to enqueue job: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to connect to job queue: {:?}", e);
                }
            }
        });

        Ok(None)
    }
}
