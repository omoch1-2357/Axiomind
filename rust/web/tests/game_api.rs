use axm_web::server::{ServerConfig, WebServer};
use serde_json::json;
use std::time::Duration;
use warp::hyper::{self, Body, Client as HyperClient, Request};

#[tokio::test]
async fn session_api_lifecycle() {
    let server = WebServer::new(ServerConfig::for_tests()).expect("construct server");
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let create_uri: hyper::Uri = format!("http://{address}/api/sessions")
        .parse()
        .expect("parse create uri");
    let create_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(create_uri.clone())
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "seed": 1337,
                "level": 2,
                "opponent_type": "human"
            })
            .to_string(),
        ))
        .expect("build create request");

    let create_response = client
        .request(create_request)
        .await
        .expect("issue create request");
    assert_eq!(
        create_response.status(),
        hyper::StatusCode::CREATED,
        "expected session creation status 201"
    );
    let create_body = hyper::body::to_bytes(create_response.into_body())
        .await
        .expect("read create body");

    // Response is now HTML with data-session-id attribute
    let html_body = String::from_utf8(create_body.to_vec()).expect("parse html");

    // Extract session_id from HTML (data-session-id="...")
    let session_id = html_body
        .split("data-session-id=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("find session_id in HTML")
        .to_string();

    let info_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}")
        .parse()
        .expect("parse info uri");
    let info_response = client.get(info_uri).await.expect("request session info");
    assert_eq!(info_response.status(), hyper::StatusCode::OK);
    let info_body = hyper::body::to_bytes(info_response.into_body())
        .await
        .expect("read info body");
    let info_json: serde_json::Value = serde_json::from_slice(&info_body).expect("parse info json");
    assert_eq!(info_json["session_id"], session_id);
    // Verify config details from the /api/sessions/{id} endpoint
    assert_eq!(info_json["config"]["seed"], 1337);
    assert_eq!(info_json["config"]["level"], 2);
    assert_eq!(info_json["config"]["opponent_type"], "human");

    let state_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}/state")
        .parse()
        .expect("parse state uri");
    let state_response = client.get(state_uri).await.expect("request state");
    assert_eq!(state_response.status(), hyper::StatusCode::OK);
    let state_body = hyper::body::to_bytes(state_response.into_body())
        .await
        .expect("read state body");
    let state_json: serde_json::Value =
        serde_json::from_slice(&state_body).expect("parse state json");
    assert_eq!(state_json["session_id"], session_id);
    // Verify state has players array
    assert!(state_json["players"].is_array());
    assert_eq!(state_json["players"].as_array().unwrap().len(), 2);

    let action_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}/actions")
        .parse()
        .expect("parse action uri");
    let action_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(action_uri.clone())
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "Check" }).to_string()))
        .expect("build action request");
    let action_response = client
        .request(action_request)
        .await
        .expect("issue action request");
    assert_eq!(action_response.status(), hyper::StatusCode::ACCEPTED);
    let action_body = hyper::body::to_bytes(action_response.into_body())
        .await
        .expect("read action body");
    let action_json: serde_json::Value =
        serde_json::from_slice(&action_body).expect("parse action json");
    assert_eq!(action_json["type"], "player_action");
    assert_eq!(action_json["session_id"], session_id);

    let delete_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}")
        .parse()
        .expect("parse delete uri");
    let delete_request = Request::builder()
        .method(hyper::Method::DELETE)
        .uri(delete_uri.clone())
        .body(Body::empty())
        .expect("build delete request");
    let delete_response = client
        .request(delete_request)
        .await
        .expect("issue delete request");
    assert_eq!(delete_response.status(), hyper::StatusCode::NO_CONTENT);

    let missing_response = client
        .get(delete_uri)
        .await
        .expect("request deleted session");
    assert_eq!(missing_response.status(), hyper::StatusCode::NOT_FOUND);

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}
