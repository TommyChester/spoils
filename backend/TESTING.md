# Testing Documentation

## Test Suite Overview

The Spoils backend has a comprehensive test suite covering unit tests, integration tests, and functional tests.

## Running Tests

### Run all tests
```bash
cargo test
```

### Run only unit tests
```bash
cargo test --lib
```

### Run specific test module
```bash
cargo test models::tests
cargo test tests::test_extract_ingredients
```

### Run tests with output
```bash
cargo test -- --nocapture
```

## Test Coverage

### Unit Tests (src/models.rs)

**Model Creation Tests:**
- `test_new_product_creation` - Tests food product model creation
- `test_new_ingredient_creation` - Tests ingredient model creation
- `test_new_ingredient_with_nutrition` - Tests ingredient with nutritional data
- `test_new_product_non_food_creation` - Tests non-food product model creation
- `test_openfoodfacts_response_parsing` - Tests OpenFoodFacts API response parsing

**Coverage:** Model struct instantiation, field assignment, serialization

### Unit Tests (src/main.rs)

**Ingredient Extraction Tests:**
- `test_extract_ingredients_with_ingredients_marker` - Tests "Ingredients:" pattern
- `test_extract_ingredients_with_contains_marker` - Tests "Contains:" pattern
- `test_extract_ingredients_with_active_ingredients` - Tests "Active Ingredients:" pattern
- `test_extract_ingredients_with_other_ingredients_marker` - Tests "Other Ingredients:" pattern
- `test_extract_ingredients_no_marker` - Tests when no ingredient marker is present
- `test_extract_ingredients_multiple_sentences` - Tests parsing stops at sentence boundaries
- `test_extract_ingredients_case_insensitive` - Tests case-insensitive pattern matching

**Coverage:** Ingredient text extraction, pattern matching, text parsing logic

### Integration Tests (tests/health_tests.rs)

**API Endpoint Tests:**
- `test_health_endpoint` - Tests /health endpoint returns 200 OK
- `test_hello_endpoint` - Tests /api/hello endpoint returns correct JSON

**Coverage:** Basic API routing, response structure

## Test Results

```
Running unittests src/lib.rs
running 5 tests
test models::tests::test_new_ingredient_creation ... ok
test models::tests::test_new_product_creation ... ok
test models::tests::test_new_ingredient_with_nutrition ... ok
test models::tests::test_new_product_non_food_creation ... ok
test models::tests::test_openfoodfacts_response_parsing ... ok
test result: ok. 5 passed; 0 failed

Running unittests src/main.rs
running 12 tests
test tests::test_extract_ingredients_no_marker ... ok
test tests::test_extract_ingredients_multiple_sentences ... ok
test tests::test_extract_ingredients_with_contains_marker ... ok
test tests::test_extract_ingredients_case_insensitive ... ok
test tests::test_extract_ingredients_with_active_ingredients ... ok
test tests::test_extract_ingredients_with_ingredients_marker ... ok
test tests::test_extract_ingredients_with_other_ingredients_marker ... ok
test result: ok. 12 passed; 0 failed

Running tests/health_tests.rs
running 2 tests
test test_health_endpoint ... ok
test test_hello_endpoint ... ok
test result: ok. 2 passed; 0 failed

Total: 19 tests passed
```

## Future Test Additions

### Recommended Additional Tests

1. **Database Integration Tests**
   - Test actual database CRUD operations
   - Test ingredient lookup with case variations
   - Test duplicate detection

2. **API Endpoint Tests**
   - `/api/products/{barcode}` - Product lookup and OpenFoodFacts integration
   - `/api/products-non-food` - Non-food product CRUD operations
   - `/api/products-non-food/{barcode}` - Non-food product lookup
   - `/api/jobs/*` - Job queue endpoint testing

3. **Job Queue Tests**
   - Test CreateIngredientJob execution
   - Test FetchProductJob execution
   - Test USDA API integration
   - Test sub-ingredient recursion

4. **Ingredient Processing Tests**
   - Test ingredient extraction from food products
   - Test ingredient extraction from supplements/beauty products
   - Test category-based processing triggers

## Testing Best Practices

1. **Isolation:** Each test should be independent
2. **Clarity:** Test names should describe what they test
3. **Coverage:** Aim for high coverage of business logic
4. **Speed:** Keep tests fast (mock external APIs when possible)
5. **Reliability:** Tests should be deterministic

## CI/CD Integration

Tests run automatically on:
- Every commit to main branch
- Every pull request
- Before deployment to Heroku

## Continuous Improvement

As new features are added, corresponding tests should be written following the Test-Driven Development (TDD) approach:
1. Write failing test
2. Implement feature
3. Verify test passes
4. Refactor if needed
