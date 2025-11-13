use actix_web::{test, App};
use backend::{health, hello};

#[actix_rt::test]
async fn test_health_endpoint() {
    let app = test::init_service(
        App::new()
            .service(health)
    ).await;

    let req = test::TestRequest::get()
        .uri("/health")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("ok"));
    assert!(body_str.contains("Spoils API is running"));
}

#[actix_rt::test]
async fn test_hello_endpoint() {
    let app = test::init_service(
        App::new()
            .service(hello)
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/hello")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains("Hello from Spoils API"));
}
