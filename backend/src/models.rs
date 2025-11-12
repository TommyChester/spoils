use diesel::prelude::*;
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
