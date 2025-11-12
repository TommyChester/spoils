CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    barcode VARCHAR(255) NOT NULL UNIQUE,
    product_name VARCHAR(1000),
    brands VARCHAR(500),
    categories TEXT,
    quantity VARCHAR(255),
    image_url TEXT,
    nutriscore_grade VARCHAR(10),
    nova_group INTEGER,
    ecoscore_grade VARCHAR(10),
    ingredients_text TEXT,
    allergens TEXT,
    full_response JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_products_barcode ON products(barcode);
CREATE INDEX idx_products_created_at ON products(created_at);
