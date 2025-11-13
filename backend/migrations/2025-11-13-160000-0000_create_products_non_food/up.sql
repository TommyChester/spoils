CREATE TABLE products_non_food (
    id SERIAL PRIMARY KEY,

    -- Basic Identification
    barcode VARCHAR(255) UNIQUE,
    upc VARCHAR(255),
    sku VARCHAR(255),
    name VARCHAR(1000) NOT NULL,
    brand VARCHAR(500),
    manufacturer VARCHAR(500),
    model_number VARCHAR(255),
    category VARCHAR(255),
    subcategory VARCHAR(255),
    description TEXT,

    -- Physical Properties
    weight_grams FLOAT,
    length_cm FLOAT,
    width_cm FLOAT,
    height_cm FLOAT,
    volume_ml FLOAT,
    color VARCHAR(255),
    material JSONB, -- Array of materials: ["Cotton", "Polyester"]
    size VARCHAR(100),

    -- Safety & Compliance
    certifications JSONB, -- ["UL Listed", "CE", "RoHS"]
    safety_warnings TEXT,
    age_restriction INTEGER, -- Minimum age
    contains_batteries BOOLEAN DEFAULT FALSE,
    hazardous_materials JSONB,
    country_of_origin VARCHAR(255),

    -- Environmental Impact
    recyclable BOOLEAN,
    recycling_info TEXT,
    eco_certifications JSONB, -- ["Energy Star", "FSC Certified"]
    sustainability_score FLOAT, -- 0-100
    carbon_footprint_kg FLOAT,
    packaging_type VARCHAR(255),
    biodegradable BOOLEAN,

    -- Usage & Care
    instructions TEXT,
    care_instructions TEXT,
    warranty_months INTEGER,
    lifespan_estimate_years FLOAT,
    maintenance_schedule TEXT,

    -- Purchase Information
    msrp_usd FLOAT,
    current_price_usd FLOAT,
    currency VARCHAR(10),
    availability VARCHAR(100), -- "In Stock", "Out of Stock", "Discontinued"
    release_date DATE,
    discontinued_date DATE,

    -- Ratings & Reviews
    average_rating FLOAT, -- 0-5 stars
    total_reviews INTEGER,

    -- Media
    images JSONB, -- Array of image URLs
    videos JSONB, -- Array of video URLs
    manuals JSONB, -- Array of manual/document URLs

    -- Additional Data
    features JSONB, -- Key features as array
    specifications JSONB, -- Technical specs as key-value pairs
    compatible_with JSONB, -- Array of compatible product IDs/names
    alternatives JSONB, -- Array of alternative product IDs/names
    tags JSONB, -- Search tags

    -- OpenProductFacts / Barcode API Data
    full_response JSONB, -- Store complete API response
    data_source VARCHAR(255), -- "OpenProductFacts", "Manual", "UPC Database"

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_verified_at TIMESTAMP
);

-- Indexes for fast searching
CREATE INDEX idx_products_nf_barcode ON products_non_food(barcode);
CREATE INDEX idx_products_nf_upc ON products_non_food(upc);
CREATE INDEX idx_products_nf_name ON products_non_food(name);
CREATE INDEX idx_products_nf_brand ON products_non_food(brand);
CREATE INDEX idx_products_nf_category ON products_non_food(category);
CREATE INDEX idx_products_nf_tags ON products_non_food USING GIN(tags);
CREATE INDEX idx_products_nf_material ON products_non_food USING GIN(material);
