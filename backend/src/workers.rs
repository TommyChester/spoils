use fang::asynk::async_queue::AsyncQueue;
use fang::asynk::async_worker_pool::AsyncWorkerPool;
use fang::NoTls;

pub async fn start_worker_pool() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    log::info!("Connecting to database for job queue: {}", database_url);

    // Create async queue
    let max_pool_size: u32 = 3;
    let mut queue = AsyncQueue::builder()
        .uri(database_url)
        .max_pool_size(max_pool_size)
        .build();

    queue.connect(NoTls).await.expect("Failed to connect to database for job queue");

    log::info!("Job queue connected successfully");

    // Start worker pool with 5 workers
    let mut pool: AsyncWorkerPool<AsyncQueue<NoTls>> = AsyncWorkerPool::builder()
        .number_of_workers(5_u32)
        .queue(queue.clone())
        .build();

    log::info!("Starting worker pool with 5 workers");

    pool.start().await;

    log::info!("Worker pool started successfully");
}
