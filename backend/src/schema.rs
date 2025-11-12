// @generated automatically by Diesel CLI.

diesel::table! {
    products (id) {
        id -> Int4,
        barcode -> Varchar,
        product_name -> Nullable<Varchar>,
        brands -> Nullable<Varchar>,
        categories -> Nullable<Text>,
        quantity -> Nullable<Varchar>,
        image_url -> Nullable<Text>,
        nutriscore_grade -> Nullable<Varchar>,
        nova_group -> Nullable<Int4>,
        ecoscore_grade -> Nullable<Varchar>,
        ingredients_text -> Nullable<Text>,
        allergens -> Nullable<Text>,
        full_response -> Jsonb,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}
