use axum::{body::Body, http::Request};
use tower::ServiceExt;

#[tokio::test]
async fn test_index_html_is_embedded() {
    let app = safepaw::server::create_ui_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), 200);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);

    // Verify it's the actual index.html content
    assert!(html.contains("SafePaw Village"));
    assert!(html.contains("pixi.min@v8.16.0.js"));
}

#[tokio::test]
async fn test_assets_are_embedded() {
    let app = safepaw::server::create_ui_router();

    // Test JavaScript file
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/app.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/javascript"
    );

    // Test grass tile asset
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/tiles/grass.png")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "image/png");

    // Test 404 for non-existent file
    let response = app
        .oneshot(
            Request::builder()
                .uri("/nonexistent.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_pixi_library_is_embedded() {
    let app = safepaw::server::create_ui_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/pixi.min@v8.16.0.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/javascript"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    // Verify it's the minified PixiJS library (should be substantial in size)
    assert!(body.len() > 100_000, "PixiJS library should be embedded");
}
