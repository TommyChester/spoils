# Job Queue System Usage Guide

## Overview

Spoils uses **fang** - a PostgreSQL-based background job processor for handling async tasks.

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌────────────┐
│   API       │ Enqueue │   fang_tasks │  Pick   │  Worker    │
│ Endpoint    ├────────►│   Table      ├────────►│   Pool     │
│             │         │  (Queue)     │         │ (5 workers)│
└─────────────┘         └──────────────┘         └────────────┘
```

## Job Types

### 1. FetchProductJob
Fetches product data from OpenFoodFacts API and stores in database.

**Features:**
- Unique execution (prevents duplicate fetches)
- 3 retries with exponential backoff (60s, 120s, 240s)
- Automatic error logging

**Usage:**
```bash
curl -X POST https://spoils-backend-82d4a06b8d67.herokuapp.com/api/jobs/fetch-product \
  -H "Content-Type: application/json" \
  -d '{"barcode": "737628064502"}'
```

### 2. AnalyzeIngredientsJob
Processes ingredient analysis for a product.

**Features:**
- Unique execution per product
- 2 retries
- Simulated 2-second processing time

**Usage:**
```bash
curl -X POST https://spoils-backend-82d4a06b8d67.herokuapp.com/api/jobs/analyze-ingredients \
  -H "Content-Type: application/json" \
  -d '{"product_id": 1}'
```

### 3. SendNotificationJob
Sends notifications (email, push, SMS, etc).

**Features:**
- Non-unique (allows multiple notifications)
- 5 retries
- 500ms execution time

**Example payload:**
```json
{
  "user_id": 123,
  "notification_type": "email",
  "message": "Your product scan is ready!"
}
```

### 4. CleanupJob
Recurring job that runs daily at 2 AM.

**Features:**
- Cron schedule: `0 2 * * *`
- Unique execution
- 1 retry
- Automatic scheduling

## API Endpoints

### Enqueue Product Fetch
```
POST /api/jobs/fetch-product
Content-Type: application/json

{
  "barcode": "737628064502"
}

Response:
{
  "message": "Job enqueued successfully",
  "barcode": "737628064502"
}
```

### Enqueue Ingredient Analysis
```
POST /api/jobs/analyze-ingredients
Content-Type: application/json

{
  "product_id": 1
}

Response:
{
  "message": "Analysis job enqueued successfully",
  "product_id": 1
}
```

### Check Queue Status
```
GET /api/jobs/status

Response:
{
  "message": "Job queue is operational",
  "status": "running"
}
```

## Database Schema

```sql
CREATE TYPE fang_task_state AS ENUM (
  'new',         -- Just enqueued
  'in_progress', -- Being processed
  'failed',      -- Failed after max retries
  'finished',    -- Completed successfully
  'retried'      -- Will be retried
);

CREATE TABLE fang_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    metadata JSONB NOT NULL,           -- Job parameters
    error_message TEXT,                -- Error details if failed
    state fang_task_state DEFAULT 'new' NOT NULL,
    task_type VARCHAR DEFAULT 'common' NOT NULL,
    uniq_hash CHAR(64),               -- For unique jobs
    retries INTEGER DEFAULT 0 NOT NULL,
    scheduled_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);
```

## Query Jobs

### View all pending jobs
```sql
SELECT id, task_type, state, retries, created_at
FROM fang_tasks
WHERE state = 'new'
ORDER BY scheduled_at;
```

### View failed jobs
```sql
SELECT id, task_type, error_message, retries, created_at
FROM fang_tasks
WHERE state = 'failed';
```

### View job history for a barcode
```sql
SELECT id, task_type, state, metadata, created_at
FROM fang_tasks
WHERE metadata->>'barcode' = '737628064502'
ORDER BY created_at DESC;
```

### Count jobs by state
```sql
SELECT state, COUNT(*)
FROM fang_tasks
GROUP BY state;
```

### View jobs by type
```sql
SELECT task_type, state, COUNT(*)
FROM fang_tasks
GROUP BY task_type, state
ORDER BY task_type, state;
```

## Creating Custom Jobs

### Step 1: Define the Job Struct

```rust
// In src/jobs.rs
use fang::{AsyncRunnable, Deserialize, FangError, Scheduled, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "fang::serde")]
pub struct YourCustomJob {
    pub your_field: String,
}
```

### Step 2: Implement AsyncRunnable

```rust
#[async_trait]
impl AsyncRunnable for YourCustomJob {
    async fn run(&self, _queue: &mut dyn AsyncQueueable) -> Result<(), FangError> {
        log::info!("Processing YourCustomJob: {}", self.your_field);

        // Your logic here

        Ok(())
    }

    fn uniq(&self) -> bool {
        true  // Set to false to allow duplicate jobs
    }

    fn task_type(&self) -> String {
        "your_custom_job".to_string()
    }

    fn cron(&self) -> Option<Scheduled> {
        None  // Or Some(Scheduled::CronPattern("0 * * * *".to_string()))
    }

    fn max_retries(&self) -> i32 {
        3
    }

    fn backoff(&self, attempt: u32) -> u32 {
        60 * (2_u32.pow(attempt))  // Exponential backoff
    }
}
```

### Step 3: Add API Endpoint

```rust
#[post("/api/jobs/your-custom-job")]
async fn enqueue_your_custom_job(
    body: web::Json<YourCustomJobRequest>,
) -> impl Responder {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let mut queue = AsyncQueue::builder()
        .uri(database_url)
        .max_pool_size(3)
        .build();

    match queue.connect(NoTls).await {
        Ok(_) => {
            let job = YourCustomJob {
                your_field: body.your_field.clone(),
            };

            match queue.insert_task(&job as &dyn AsyncRunnable).await {
                Ok(_) => {
                    HttpResponse::Ok().json(serde_json::json!({
                        "message": "Job enqueued successfully"
                    }))
                }
                Err(e) => {
                    HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to enqueue job"
                    }))
                }
            }
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to connect to job queue"
            }))
        }
    }
}
```

### Step 4: Register the Endpoint

```rust
// In main.rs
.service(enqueue_your_custom_job)
```

## Worker Pool Configuration

The worker pool is configured in `src/workers.rs`:

```rust
let mut pool: AsyncWorkerPool<AsyncQueue<NoTls>> = AsyncWorkerPool::builder()
    .number_of_workers(5_u32)  // Adjust based on load
    .queue(queue.clone())
    .build();
```

**Tuning:**
- Increase workers for higher throughput
- Decrease for lower resource usage
- Monitor with `heroku ps` on Heroku

## Monitoring

### Heroku Logs
```bash
heroku logs --tail -a spoils-backend | grep -E "Processing|Enqueued|Failed"
```

### Job Queue Dashboard (PostgreSQL)
```sql
-- Active jobs
SELECT COUNT(*) FROM fang_tasks WHERE state = 'in_progress';

-- Pending jobs
SELECT COUNT(*) FROM fang_tasks WHERE state = 'new';

-- Failed jobs (last hour)
SELECT COUNT(*) FROM fang_tasks
WHERE state = 'failed'
  AND created_at > NOW() - INTERVAL '1 hour';

-- Average processing time by job type
SELECT
    task_type,
    AVG(EXTRACT(EPOCH FROM (updated_at - created_at))) as avg_seconds
FROM fang_tasks
WHERE state = 'finished'
GROUP BY task_type;
```

## Retry Strategy

Jobs are retried with exponential backoff:

| Attempt | Delay | Total Wait |
|---------|-------|------------|
| 1st     | 0s    | 0s         |
| 2nd     | 60s   | 60s        |
| 3rd     | 120s  | 180s       |
| 4th     | 240s  | 420s       |

After `max_retries`, job is marked as `failed`.

## Best Practices

1. **Keep jobs idempotent**: Jobs may be retried, ensure they can run multiple times safely
2. **Use unique jobs for deduplication**: Set `uniq() = true` for jobs that shouldn't run twice
3. **Log extensively**: Use `log::info!()` and `log::error!()` for debugging
4. **Handle errors gracefully**: Return `FangError` with descriptive messages
5. **Monitor failed jobs**: Set up alerts for jobs stuck in `failed` state
6. **Clean up old jobs**: Periodically delete old finished/failed jobs

## Troubleshooting

### Jobs not processing
1. Check worker pool is running: `heroku logs -a spoils-backend | grep "Worker pool"`
2. Check database connection: `heroku pg:info -a spoils-backend`
3. Check for errors: `SELECT * FROM fang_tasks WHERE state = 'failed' LIMIT 10;`

### High retry rate
1. Check error messages: `SELECT error_message, COUNT(*) FROM fang_tasks WHERE state = 'failed' GROUP BY error_message;`
2. Increase backoff times in job implementation
3. Check external API rate limits

### Memory issues
1. Reduce number of workers in `src/workers.rs`
2. Check Heroku dyno metrics: `heroku ps -a spoils-backend`
3. Upgrade dyno if needed: `heroku ps:resize worker=standard-2x -a spoils-backend`
