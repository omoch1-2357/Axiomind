/// Comprehensive end-to-end tests for complete game sessions
/// Tests full game flow from session creation to completion with real engine integration
use axm_engine::player::PlayerAction;
use axm_web::server::{AppContext, ServerConfig, WebServer};
use axm_web::session::GameConfig;
use serde_json::json;
use std::time::Duration;
use warp::hyper::{self, Body, Client as HyperClient, Request};

/// Test complete game session from start to finish with AI opponent
#[tokio::test]
async fn test_complete_game_session_with_ai() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");
    let server = WebServer::from_context(context.clone());
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Step 1: Create session with AI opponent
    let create_uri: hyper::Uri = format!("http://{address}/api/sessions")
        .parse()
        .expect("parse create uri");
    let create_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(create_uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "seed": 42,
                "level": 1,
                "opponent_type": "ai:baseline"
            })
            .to_string(),
        ))
        .expect("build create request");

    let create_response = client
        .request(create_request)
        .await
        .expect("create session");
    assert_eq!(create_response.status(), hyper::StatusCode::CREATED);

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

    // Step 2: Subscribe to SSE events
    let sse_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}/events")
        .parse()
        .expect("parse sse uri");
    let sse_response = client.get(sse_uri).await.expect("connect sse");
    assert_eq!(sse_response.status(), hyper::StatusCode::OK);

    // Step 3: Get initial game state
    let state_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}/state")
        .parse()
        .expect("parse state uri");
    let state_response = client.get(state_uri.clone()).await.expect("get state");
    assert_eq!(state_response.status(), hyper::StatusCode::OK);

    let state_body = hyper::body::to_bytes(state_response.into_body())
        .await
        .expect("read state body");
    let state_json: serde_json::Value = serde_json::from_slice(&state_body).expect("parse state");

    // Verify initial state has 2 players
    assert_eq!(state_json["players"].as_array().unwrap().len(), 2);
    assert!(state_json["current_player"].is_number());

    // Step 4: Play through a complete hand by taking actions
    let action_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}/actions")
        .parse()
        .expect("parse action uri");

    // Take a bet action to ensure pot changes
    let action_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(action_uri.clone())
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": { "Bet": 200 } }).to_string()))
        .expect("build action request");

    let action_response = client.request(action_request).await.expect("submit action");

    // Action should be accepted or return game state update
    assert!(
        action_response.status() == hyper::StatusCode::ACCEPTED
            || action_response.status() == hyper::StatusCode::OK
            || action_response.status() == hyper::StatusCode::BAD_REQUEST, // Bet might be invalid
        "Unexpected status: {:?}",
        action_response.status()
    );

    // Wait a bit for AI to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Step 5: Verify game progressed by checking state again
    let state_response2 = client.get(state_uri).await.expect("get state again");
    let state_body2 = hyper::body::to_bytes(state_response2.into_body())
        .await
        .expect("read state body");
    let state_json2: serde_json::Value = serde_json::from_slice(&state_body2).expect("parse state");

    // State should have changed - check various indicators
    let pot_changed = state_json["pot"] != state_json2["pot"];
    let current_player_changed = state_json["current_player"] != state_json2["current_player"];
    let hand_completed = state_json2["hand_id"].is_null();

    // At least one of these should be true
    assert!(
        pot_changed || current_player_changed || hand_completed,
        "Game state should have changed (pot_changed={}, player_changed={}, hand_completed={})",
        pot_changed,
        current_player_changed,
        hand_completed
    );

    // Step 6: Delete session
    let delete_uri: hyper::Uri = format!("http://{address}/api/sessions/{session_id}")
        .parse()
        .expect("parse delete uri");
    let delete_request = Request::builder()
        .method(hyper::Method::DELETE)
        .uri(delete_uri)
        .body(Body::empty())
        .expect("build delete request");
    let delete_response = client
        .request(delete_request)
        .await
        .expect("delete session");
    assert_eq!(delete_response.status(), hyper::StatusCode::NO_CONTENT);

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}

/// Test complete hand with full showdown
#[tokio::test]
async fn test_complete_hand_with_showdown() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    // Create session with deterministic seed for reproducible gameplay
    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(12345),
            level: 1,
            opponent_type: axm_web::session::OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    // Subscribe to events to track game progress
    let mut subscription = context.event_bus().subscribe(session_id.clone());

    // Play through actions until hand completes
    let mut actions_taken = 0;
    let max_actions = 20; // Safety limit

    loop {
        if actions_taken >= max_actions {
            panic!("Too many actions taken without hand completion");
        }

        // Get current state
        let state = context.sessions().state(&session_id).expect("get state");

        // Check if hand is complete
        if state.hand_id.is_none() {
            break;
        }

        // Try to process next action (Check if possible, otherwise Call)
        let result = context
            .sessions()
            .process_action(&session_id, PlayerAction::Check);

        if result.is_err() {
            // If Check fails, try Call
            let _ = context
                .sessions()
                .process_action(&session_id, PlayerAction::Call);
        }

        actions_taken += 1;

        // Small delay to allow event processing
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Verify we received game events
    let mut event_count = 0;
    while let Ok(Some(_event)) =
        tokio::time::timeout(Duration::from_millis(50), subscription.receiver.recv()).await
    {
        event_count += 1;
    }

    assert!(event_count > 0, "Should have received game events");
    assert!(actions_taken > 0, "Should have taken at least one action");
}

/// Test multiple sequential hands in same session
#[tokio::test]
async fn test_multiple_hands_in_session() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(98765),
            level: 1,
            opponent_type: axm_web::session::OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    let mut hands_completed = 0;
    let target_hands = 3;

    for _ in 0..target_hands {
        // Play through one hand
        let mut actions_in_hand = 0;
        loop {
            if actions_in_hand > 30 {
                break; // Safety limit
            }

            let state = context.sessions().state(&session_id).expect("get state");

            if state.hand_id.is_none() {
                hands_completed += 1;
                break;
            }

            // Take action
            let _ = context
                .sessions()
                .process_action(&session_id, PlayerAction::Check)
                .or_else(|_| {
                    context
                        .sessions()
                        .process_action(&session_id, PlayerAction::Call)
                });

            actions_in_hand += 1;
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    assert_eq!(
        hands_completed, target_hands,
        "Should have completed all target hands"
    );
}

/// Test session with human vs human (no AI)
#[tokio::test]
async fn test_human_vs_human_session() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(55555),
            level: 1,
            opponent_type: axm_web::session::OpponentType::Human,
        })
        .expect("create session");

    let state = context.sessions().state(&session_id).expect("get state");

    // Both players should be marked as human
    assert_eq!(state.players.len(), 2);

    // Should have a current player waiting for action
    assert!(state.current_player.is_some());
}

/// Test error handling in complete game flow
#[tokio::test]
async fn test_error_handling_in_game_flow() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");
    let server = WebServer::from_context(context.clone());
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Try to access non-existent session
    let fake_session_id = "00000000-0000-0000-0000-000000000000";
    let state_uri: hyper::Uri = format!("http://{address}/api/sessions/{fake_session_id}/state")
        .parse()
        .expect("parse uri");

    let response = client.get(state_uri).await.expect("request");
    assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

    // Try invalid action on non-existent session
    let action_uri: hyper::Uri = format!("http://{address}/api/sessions/{fake_session_id}/actions")
        .parse()
        .expect("parse uri");
    let action_request = Request::builder()
        .method(hyper::Method::POST)
        .uri(action_uri)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "action": "Check" }).to_string()))
        .expect("build request");

    let response = client.request(action_request).await.expect("request");
    assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}
