use axm_web::{AppContext, AppSettings, ServerConfig, WebServer};
use warp::hyper::{self, Body, Client as HyperClient, Request};

#[tokio::test]
async fn settings_api_get_returns_defaults() {
    let config = ServerConfig::for_tests();
    let server = WebServer::new(config).expect("create server");
    let handle = server.start().await.expect("start server");
    let addr = handle.address();
    let client = HyperClient::new();

    let uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");

    let request = Request::builder()
        .method(hyper::Method::GET)
        .uri(uri)
        .body(Body::empty())
        .expect("build request");

    let response = client.request(request).await.expect("send request");
    assert_eq!(response.status(), hyper::StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("read body");
    let settings: AppSettings = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(settings, AppSettings::default());

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn settings_api_update_modifies_values() {
    let config = ServerConfig::for_tests();
    let server = WebServer::new(config).expect("create server");
    let handle = server.start().await.expect("start server");
    let addr = handle.address();
    let client = HyperClient::new();

    // Update settings
    let update_body = serde_json::json!({
        "default_level": 5,
        "default_ai_strategy": "aggressive",
        "session_timeout_minutes": 60
    });

    let uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");

    let request = Request::builder()
        .method(hyper::Method::PUT)
        .uri(uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(update_body.to_string()))
        .expect("build request");

    let response = client.request(request).await.expect("send request");
    assert_eq!(response.status(), hyper::StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("read body");
    let settings: AppSettings = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(settings.default_level, 5);
    assert_eq!(settings.default_ai_strategy, "aggressive");
    assert_eq!(settings.session_timeout_minutes, 60);

    // Verify changes persist
    let get_uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");
    let get_request = Request::builder()
        .method(hyper::Method::GET)
        .uri(get_uri)
        .body(Body::empty())
        .expect("build request");

    let get_response = client.request(get_request).await.expect("send request");
    let get_body = hyper::body::to_bytes(get_response.into_body())
        .await
        .expect("read body");
    let retrieved: AppSettings = serde_json::from_slice(&get_body).expect("parse json");
    assert_eq!(retrieved.default_level, 5);

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn settings_api_validates_updates() {
    let config = ServerConfig::for_tests();
    let server = WebServer::new(config).expect("create server");
    let handle = server.start().await.expect("start server");
    let addr = handle.address();
    let client = HyperClient::new();

    // Invalid level
    let invalid_body = serde_json::json!({
        "default_level": 99
    });

    let uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");

    let request = Request::builder()
        .method(hyper::Method::PUT)
        .uri(uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(invalid_body.to_string()))
        .expect("build request");

    let response = client.request(request).await.expect("send request");
    assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

    // Settings should remain at defaults
    let get_uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");
    let get_request = Request::builder()
        .method(hyper::Method::GET)
        .uri(get_uri)
        .body(Body::empty())
        .expect("build request");

    let get_response = client.request(get_request).await.expect("send request");
    let body = hyper::body::to_bytes(get_response.into_body())
        .await
        .expect("read body");
    let settings: AppSettings = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(settings, AppSettings::default());

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn settings_api_update_field_changes_single_value() {
    let config = ServerConfig::for_tests();
    let server = WebServer::new(config).expect("create server");
    let handle = server.start().await.expect("start server");
    let addr = handle.address();
    let client = HyperClient::new();

    let update_body = serde_json::json!({
        "field": "default_level",
        "value": 3
    });

    let uri: hyper::Uri = format!("http://{}/api/settings/field", addr)
        .parse()
        .expect("parse uri");

    let request = Request::builder()
        .method(hyper::Method::PATCH)
        .uri(uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(update_body.to_string()))
        .expect("build request");

    let response = client.request(request).await.expect("send request");
    assert_eq!(response.status(), hyper::StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body())
        .await
        .expect("read body");
    let settings: AppSettings = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(settings.default_level, 3);
    assert_eq!(settings.default_ai_strategy, "baseline"); // Unchanged

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn settings_api_reset_restores_defaults() {
    let config = ServerConfig::for_tests();
    let server = WebServer::new(config).expect("create server");
    let handle = server.start().await.expect("start server");
    let addr = handle.address();
    let client = HyperClient::new();

    // Modify settings first
    let update_body = serde_json::json!({
        "default_level": 8
    });

    let update_uri: hyper::Uri = format!("http://{}/api/settings", addr)
        .parse()
        .expect("parse uri");
    let update_request = Request::builder()
        .method(hyper::Method::PUT)
        .uri(update_uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(update_body.to_string()))
        .expect("build request");

    client.request(update_request).await.expect("send request");

    // Reset
    let reset_uri: hyper::Uri = format!("http://{}/api/settings/reset", addr)
        .parse()
        .expect("parse uri");
    let reset_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(reset_uri)
        .body(Body::empty())
        .expect("build request");

    let reset_response = client.request(reset_request).await.expect("send request");
    assert_eq!(reset_response.status(), hyper::StatusCode::OK);

    let body = hyper::body::to_bytes(reset_response.into_body())
        .await
        .expect("read body");
    let settings: AppSettings = serde_json::from_slice(&body).expect("parse json");
    assert_eq!(settings, AppSettings::default());

    handle.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn settings_store_integration_with_context() {
    let context = AppContext::new_for_tests();
    let settings = context.settings();

    // Verify default state
    let current = settings.get().expect("get settings");
    assert_eq!(current, AppSettings::default());

    // Update through store
    let new_settings = AppSettings {
        default_level: 7,
        ..Default::default()
    };

    settings.update(new_settings).expect("update settings");

    // Verify update
    let updated = settings.get().expect("get settings");
    assert_eq!(updated.default_level, 7);
}
