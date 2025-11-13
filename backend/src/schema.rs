// @generated automatically by Diesel CLI.

diesel::table! {
    ingredients (id) {
        id -> Int4,
        name -> Varchar,
        branded -> Bool,
        sub_ingredients -> Array<Int4>,
        parent_ingredients -> Array<Int4>,
        gram_protein_per_gram -> Nullable<Float4>,
        gram_carbs_per_gram -> Nullable<Float4>,
        gram_fat_per_gram -> Nullable<Float4>,
        gram_fiber_per_gram -> Nullable<Float4>,
        vitamins -> Nullable<Jsonb>,
        minerals -> Nullable<Jsonb>,
        essential_fatty_acids -> Nullable<Jsonb>,
        essential_amino_acids -> Nullable<Jsonb>,
        heavy_metals -> Nullable<Jsonb>,
        micro_plastics -> Nullable<Jsonb>,
        industrial_chemicals -> Nullable<Jsonb>,
        pesticides -> Nullable<Jsonb>,
        hormones -> Nullable<Jsonb>,
        antibiotics -> Nullable<Jsonb>,
        beta_agonists -> Nullable<Jsonb>,
        antiparasitics -> Nullable<Jsonb>,
        carcinogens -> Nullable<Jsonb>,
        natural_toxins -> Nullable<Jsonb>,
        radiological -> Nullable<Jsonb>,
        historical_issues -> Nullable<Jsonb>,
        fraudulent_ingredients -> Nullable<Jsonb>,
        dyes -> Nullable<Jsonb>,
        emulsifiers -> Nullable<Jsonb>,
        preservatives -> Nullable<Jsonb>,
        gram_trans_fat_per_gram -> Nullable<Float4>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

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

diesel::allow_tables_to_appear_in_same_query!(
    ingredients,
    products,
);
