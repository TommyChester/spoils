CREATE TABLE ingredients (
    id SERIAL PRIMARY KEY,
    name VARCHAR(500) NOT NULL,
    branded BOOLEAN NOT NULL DEFAULT FALSE,
    sub_ingredients INTEGER[] DEFAULT '{}',
    parent_ingredients INTEGER[] DEFAULT '{}',
    gram_protein_per_gram FLOAT,
    gram_carbs_per_gram FLOAT,
    gram_fat_per_gram FLOAT,
    gram_fiber_per_gram FLOAT,
    vitamins JSONB,
    minerals JSONB,
    essential_fatty_acids JSONB,
    essential_amino_acids JSONB,
    heavy_metals JSONB,
    micro_plastics JSONB,
    industrial_chemicals JSONB,
    pesticides JSONB,
    hormones JSONB,
    antibiotics JSONB,
    beta_agonists JSONB,
    antiparasitics JSONB,
    carcinogens JSONB,
    natural_toxins JSONB,
    radiological JSONB,
    historical_issues JSONB,
    fraudulent_ingredients JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_ingredients_name ON ingredients(name);
CREATE INDEX idx_ingredients_branded ON ingredients(branded);
CREATE INDEX idx_ingredients_sub_ingredients ON ingredients USING GIN(sub_ingredients);
CREATE INDEX idx_ingredients_parent_ingredients ON ingredients USING GIN(parent_ingredients);
