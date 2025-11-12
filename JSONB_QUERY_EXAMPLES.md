# JSONB Query Examples for Products Table

## Table Structure

The `products` table stores the complete OpenFoodFacts response (1,959 lines, ~15KB) in the `full_response` JSONB column.

## Size Information

```sql
SELECT
  barcode,
  product_name,
  pg_column_size(full_response) as bytes,
  pg_size_pretty(pg_column_size(full_response)) as size
FROM products;
```

## Basic Field Access

```sql
-- Access top-level fields
SELECT
  full_response->>'product_name' as name,
  full_response->>'brands' as brands,
  full_response->>'nutriscore_grade' as nutriscore,
  full_response->>'nova_group' as nova
FROM products;
```

## Nested Object Access

```sql
-- Access nutrition data
SELECT
  product_name,
  full_response->'nutriments'->>'energy-kcal_100g' as calories,
  full_response->'nutriments'->>'proteins_100g' as protein,
  full_response->'nutriments'->>'sugars_100g' as sugars,
  full_response->'nutriments'->>'fat_100g' as fat,
  full_response->'nutriments'->>'fiber_100g' as fiber,
  full_response->'nutriments'->>'salt_100g' as salt
FROM products;
```

## Array Access

```sql
-- Count ingredients
SELECT
  product_name,
  jsonb_array_length(full_response->'ingredients') as ingredient_count
FROM products;

-- Get first ingredient
SELECT
  product_name,
  full_response->'ingredients'->0->>'text' as first_ingredient
FROM products;

-- Extract all allergen tags
SELECT
  product_name,
  full_response->'allergens_tags' as allergens
FROM products;
```

## Deep Nested Access

```sql
-- Environmental score details
SELECT
  product_name,
  full_response->'ecoscore_data'->'adjustments'->'origins_of_ingredients'->>'epi_value' as environmental_impact,
  full_response->'ecoscore_data'->'adjustments'->'packaging'->>'value' as packaging_score
FROM products;

-- Nutriscore breakdown
SELECT
  product_name,
  full_response->'nutriscore'->'2023'->'data'->>'score' as score,
  full_response->'nutriscore'->'2023'->'data'->>'grade' as grade,
  full_response->'nutriscore'->'2023'->'data'->'components'->'negative'->0->>'value' as energy_value,
  full_response->'nutriscore'->'2023'->'data'->'components'->'negative'->1->>'value' as sugar_value
FROM products;
```

## Search and Filter

```sql
-- Find products with high sugar
SELECT product_name, brands
FROM products
WHERE (full_response->'nutriments'->>'sugars_100g')::float > 10;

-- Find vegan products
SELECT product_name, brands
FROM products
WHERE full_response->'labels_tags' @> '["en:vegan"]';

-- Find products from specific country
SELECT product_name, brands
FROM products
WHERE full_response->'origins_tags' @> '["en:thailand"]';

-- Find products with specific allergen
SELECT product_name, allergens
FROM products
WHERE full_response->'allergens_tags' @> '["en:peanuts"]';
```

## Image URLs

```sql
-- Get all image URLs
SELECT
  product_name,
  full_response->>'image_front_url' as front_image,
  full_response->>'image_ingredients_url' as ingredients_image,
  full_response->>'image_nutrition_url' as nutrition_image
FROM products;

-- Get specific image size
SELECT
  product_name,
  full_response->'images'->'front_en'->'sizes'->'400'->>'h' as height,
  full_response->'images'->'front_en'->'sizes'->'400'->>'w' as width
FROM products;
```

## Aggregations

```sql
-- Average calories by nova group
SELECT
  full_response->>'nova_group' as nova_group,
  AVG((full_response->'nutriments'->>'energy-kcal_100g')::float) as avg_calories
FROM products
GROUP BY full_response->>'nova_group';

-- Count products by nutriscore grade
SELECT
  full_response->>'nutriscore_grade' as grade,
  COUNT(*) as count
FROM products
GROUP BY full_response->>'nutriscore_grade'
ORDER BY grade;
```

## Check if Key Exists

```sql
-- Find products with ecoscore data
SELECT product_name
FROM products
WHERE full_response ? 'ecoscore_grade'
  AND full_response->>'ecoscore_grade' != 'unknown';
```

## Update JSONB Fields

```sql
-- Add a custom field
UPDATE products
SET full_response = full_response || '{"custom_field": "custom_value"}'::jsonb
WHERE barcode = '737628064502';

-- Update nested value
UPDATE products
SET full_response = jsonb_set(
  full_response,
  '{custom_notes}',
  '"This is a test note"'::jsonb
)
WHERE barcode = '737628064502';
```

## Performance Tips

1. **Create GIN Index for searches:**
```sql
CREATE INDEX idx_products_full_response_gin ON products USING GIN (full_response);
```

2. **Index specific paths:**
```sql
CREATE INDEX idx_nutriscore ON products ((full_response->>'nutriscore_grade'));
CREATE INDEX idx_nova_group ON products ((full_response->>'nova_group'));
```

3. **Use EXPLAIN to check query performance:**
```sql
EXPLAIN ANALYZE
SELECT * FROM products
WHERE full_response->'allergens_tags' @> '["en:peanuts"]';
```

## Pretty Print Full Response

```sql
SELECT
  barcode,
  product_name,
  jsonb_pretty(full_response)
FROM products
LIMIT 1;
```

## Extract Specific Section

```sql
-- Get just the ingredients array
SELECT
  product_name,
  jsonb_pretty(full_response->'ingredients') as ingredients_detail
FROM products
WHERE barcode = '737628064502';

-- Get just nutriments
SELECT
  product_name,
  jsonb_pretty(full_response->'nutriments') as nutrition_facts
FROM products;
```
