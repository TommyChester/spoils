use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, NaiveDate};

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
    pub gram_protein_per_gram: Option<f32>,
    pub gram_carbs_per_gram: Option<f32>,
    pub gram_fat_per_gram: Option<f32>,
    pub gram_fiber_per_gram: Option<f32>,
}

impl Ingredient {
    /// Find ingredient by name (case-insensitive) in database only
    /// Returns Option<i32> - ingredient ID if found, None if not found
    pub fn find_in_db(
        ingredient_name: &str,
        conn: &mut PgConnection,
    ) -> Result<Option<i32>, diesel::result::Error> {
        use crate::schema::ingredients::dsl::*;
        use diesel::dsl::sql;
        use diesel::sql_types::Bool;

        // Try to find with case-insensitive search
        let found = ingredients
            .filter(sql::<Bool>(&format!("LOWER(name) = LOWER('{}')", ingredient_name.replace("'", "''"))))
            .select(id)
            .first::<i32>(conn)
            .optional()?;

        if let Some(ingredient_id) = found {
            log::info!("Found existing ingredient: {} (ID: {})", ingredient_name, ingredient_id);
        }

        Ok(found)
    }

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

        // Spawn async task to enqueue job (don't block the current thread)
        let ingredient_name_clone = ingredient_name.to_string();
        tokio::spawn(async move {
            let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

            let mut queue = AsyncQueue::builder()
                .uri(database_url)
                .max_pool_size(1_u32)  // Use small pool size to avoid overwhelming DB
                .build();

            // Use timeout for connection to avoid blocking forever
            let connect_result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                queue.connect(NoTls)
            ).await;

            match connect_result {
                Ok(Ok(_)) => {
                    let job = CreateIngredientJob {
                        name: ingredient_name_clone.clone(),
                    };

                    match queue.insert_task(&job).await {
                        Ok(_) => {
                            log::info!("Successfully enqueued CreateIngredientJob for '{}'", ingredient_name_clone);
                        }
                        Err(e) => {
                            log::error!("Failed to enqueue job for '{}': {:?}", ingredient_name_clone, e);
                        }
                    }
                }
                Ok(Err(e)) => {
                    log::error!("Failed to connect to job queue for '{}': {:?}", ingredient_name_clone, e);
                }
                Err(_) => {
                    log::error!("Timeout connecting to job queue for '{}'", ingredient_name_clone);
                }
            }
        });

        Ok(None)
    }
}

// ============= Non-Food Products =============

#[derive(Queryable, Serialize, Selectable, Debug)]
#[diesel(table_name = crate::schema::products_non_food)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ProductNonFood {
    pub id: i32,
    pub barcode: Option<String>,
    pub upc: Option<String>,
    pub sku: Option<String>,
    pub name: String,
    pub brand: Option<String>,
    pub manufacturer: Option<String>,
    pub model_number: Option<String>,
    pub category: Option<String>,
    pub subcategory: Option<String>,
    pub description: Option<String>,
    pub weight_grams: Option<f32>,
    pub length_cm: Option<f32>,
    pub width_cm: Option<f32>,
    pub height_cm: Option<f32>,
    pub volume_ml: Option<f32>,
    pub color: Option<String>,
    pub material: Option<serde_json::Value>,
    pub size: Option<String>,
    pub certifications: Option<serde_json::Value>,
    pub safety_warnings: Option<String>,
    pub age_restriction: Option<i32>,
    pub contains_batteries: Option<bool>,
    pub hazardous_materials: Option<serde_json::Value>,
    pub country_of_origin: Option<String>,
    pub recyclable: Option<bool>,
    pub recycling_info: Option<String>,
    pub eco_certifications: Option<serde_json::Value>,
    pub sustainability_score: Option<f32>,
    pub carbon_footprint_kg: Option<f32>,
    pub packaging_type: Option<String>,
    pub biodegradable: Option<bool>,
    pub instructions: Option<String>,
    pub care_instructions: Option<String>,
    pub warranty_months: Option<i32>,
    pub lifespan_estimate_years: Option<f32>,
    pub maintenance_schedule: Option<String>,
    pub msrp_usd: Option<f32>,
    pub current_price_usd: Option<f32>,
    pub currency: Option<String>,
    pub availability: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub discontinued_date: Option<NaiveDate>,
    pub average_rating: Option<f32>,
    pub total_reviews: Option<i32>,
    pub images: Option<serde_json::Value>,
    pub videos: Option<serde_json::Value>,
    pub manuals: Option<serde_json::Value>,
    pub features: Option<serde_json::Value>,
    pub specifications: Option<serde_json::Value>,
    pub compatible_with: Option<serde_json::Value>,
    pub alternatives: Option<serde_json::Value>,
    pub tags: Option<serde_json::Value>,
    pub full_response: Option<serde_json::Value>,
    pub data_source: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub last_verified_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::products_non_food)]
pub struct NewProductNonFood {
    pub barcode: Option<String>,
    pub name: String,
    pub brand: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub full_response: Option<serde_json::Value>,
    pub data_source: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_product_creation() {
        let product = NewProduct {
            barcode: "123456789".to_string(),
            product_name: Some("Test Product".to_string()),
            brands: Some("Test Brand".to_string()),
            categories: None,
            quantity: None,
            image_url: None,
            nutriscore_grade: None,
            nova_group: None,
            ecoscore_grade: None,
            ingredients_text: Some("water, salt".to_string()),
            allergens: None,
            full_response: serde_json::json!({}),
        };

        assert_eq!(product.barcode, "123456789");
        assert_eq!(product.product_name, Some("Test Product".to_string()));
        assert_eq!(product.brands, Some("Test Brand".to_string()));
    }

    #[test]
    fn test_new_ingredient_creation() {
        let ingredient = NewIngredient {
            name: "Salt".to_string(),
            branded: false,
            gram_protein_per_gram: None,
            gram_carbs_per_gram: None,
            gram_fat_per_gram: None,
            gram_fiber_per_gram: None,
        };

        assert_eq!(ingredient.name, "Salt");
        assert_eq!(ingredient.branded, false);
    }

    #[test]
    fn test_new_ingredient_with_nutrition() {
        let ingredient = NewIngredient {
            name: "Chicken Breast".to_string(),
            branded: false,
            gram_protein_per_gram: Some(0.31),
            gram_carbs_per_gram: Some(0.0),
            gram_fat_per_gram: Some(0.037),
            gram_fiber_per_gram: Some(0.0),
        };

        assert_eq!(ingredient.name, "Chicken Breast");
        assert_eq!(ingredient.gram_protein_per_gram, Some(0.31));
        assert_eq!(ingredient.gram_carbs_per_gram, Some(0.0));
    }

    #[test]
    fn test_new_product_non_food_creation() {
        let product = NewProductNonFood {
            barcode: Some("999888777".to_string()),
            name: "Test Supplement".to_string(),
            brand: Some("Health Co".to_string()),
            category: Some("Supplements".to_string()),
            description: Some("Ingredients: Vitamin C, Zinc".to_string()),
            full_response: None,
            data_source: Some("Manual".to_string()),
        };

        assert_eq!(product.name, "Test Supplement");
        assert_eq!(product.category, Some("Supplements".to_string()));
        assert!(product.description.unwrap().contains("Vitamin C"));
    }

    #[test]
    fn test_openfoodfacts_response_parsing() {
        let json_data = r#"{
            "status": 1,
            "code": "3017620422003",
            "product": {
                "product_name": "Nutella"
            }
        }"#;

        let response: OpenFoodFactsResponse = serde_json::from_str(json_data).unwrap();
        assert_eq!(response.status, 1);
        assert_eq!(response.code, Some("3017620422003".to_string()));
        assert!(response.product.is_some());
    }
}
