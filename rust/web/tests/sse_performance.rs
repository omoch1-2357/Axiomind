/// Performance tests for SSE and API endpoints
/// Tests throughput, latency, and resource usage under various loads
use axm_web::events::GameEvent;
use axm_web::server::{AppContext, ServerConfig, WebServer};
use axm_web::session::{GameConfig, OpponentType};
use serde_json::json;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use warp::hyper::{self, Body, Client as HyperClient, Request};

/// Test SSE event delivery latency
#[tokio::test]
async fn test_sse_event_latency() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_id = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create session");

    let mut subscription = context.event_bus().subscribe(session_id.clone());

    let mut latencies = Vec::new();

    for i in 0..100 {
        let start = Instant::now();

        context.event_bus().broadcast(
            &session_id,
            GameEvent::Error {
                session_id: session_id.clone(),
                message: format!("latency-test-{}", i),
            },
        );

        // Wait for event
        tokio::time::timeout(Duration::from_millis(100), subscription.receiver.recv())
            .await
            .expect("receive timeout")
            .expect("channel closed");

        let latency = start.elapsed();
        latencies.push(latency);
    }

    // Calculate statistics
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let max_latency = latencies.iter().max().unwrap();

    println!("Average SSE latency: {:?}", avg_latency);
    println!("Max SSE latency: {:?}", max_latency);

    // Latency should be reasonable (under 10ms average)
    assert!(
        avg_latency < Duration::from_millis(10),
        "Average latency too high: {:?}",
        avg_latency
    );
    assert!(
        *max_latency < Duration::from_millis(50),
        "Max latency too high: {:?}",
        max_latency
    );
}

/// Test SSE throughput with many events
#[tokio::test]
async fn test_sse_throughput() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_id = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create session");

    let mut subscription = context.event_bus().subscribe(session_id.clone());

    let event_count = 1000;
    let start = Instant::now();

    // Broadcast many events
    for i in 0..event_count {
        context.event_bus().broadcast(
            &session_id,
            GameEvent::Error {
                session_id: session_id.clone(),
                message: format!("throughput-{}", i),
            },
        );
    }

    // Receive all events
    let mut received = 0;
    while received < event_count {
        if tokio::time::timeout(Duration::from_millis(10), subscription.receiver.recv())
            .await
            .is_ok()
        {
            received += 1;
        } else {
            break;
        }
    }

    let duration = start.elapsed();
    let events_per_sec = (received as f64 / duration.as_secs_f64()) as u64;

    println!(
        "SSE Throughput: {} events in {:?} ({} events/sec)",
        received, duration, events_per_sec
    );

    // Should handle at least 10,000 events per second
    assert!(
        events_per_sec > 10_000,
        "Throughput too low: {} events/sec",
        events_per_sec
    );
}

/// Test multiple concurrent SSE connections performance
#[tokio::test]
async fn test_concurrent_sse_connections_performance() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let session_id = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create session");

    let subscriber_count = 50;
    let events_per_subscriber = 20;

    let start = Instant::now();

    // Create many subscribers
    let mut subscriptions = Vec::new();
    for _ in 0..subscriber_count {
        subscriptions.push(context.event_bus().subscribe(session_id.clone()));
    }

    let subscribe_duration = start.elapsed();
    println!(
        "Created {} subscriptions in {:?}",
        subscriber_count, subscribe_duration
    );

    // Broadcast events
    let broadcast_start = Instant::now();
    for i in 0..events_per_subscriber {
        context.event_bus().broadcast(
            &session_id,
            GameEvent::Error {
                session_id: session_id.clone(),
                message: format!("event-{}", i),
            },
        );
    }
    let broadcast_duration = broadcast_start.elapsed();

    println!(
        "Broadcasted {} events to {} subscribers in {:?}",
        events_per_subscriber, subscriber_count, broadcast_duration
    );

    // Verify all subscribers received events
    let mut total_received = 0;
    for mut sub in subscriptions {
        let mut count = 0;
        while let Ok(Some(_)) =
            tokio::time::timeout(Duration::from_millis(10), sub.receiver.recv()).await
        {
            count += 1;
        }
        total_received += count;
    }

    let expected = subscriber_count * events_per_subscriber;
    assert_eq!(
        total_received, expected,
        "All subscribers should receive all events"
    );

    // Total time should be reasonable (under 1 second)
    let total_duration = start.elapsed();
    assert!(
        total_duration < Duration::from_secs(1),
        "Total time too long: {:?}",
        total_duration
    );
}

/// Test API endpoint response times
#[tokio::test]
async fn test_api_endpoint_response_times() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");
    let server = WebServer::from_context(context.clone());
    let handle = server.start().await.expect("start server");
    let address = handle.address();
    let client = HyperClient::new();

    tokio::time::sleep(Duration::from_millis(20)).await;

    // Test session creation time
    let mut create_times = Vec::new();
    for i in 0..50 {
        let start = Instant::now();

        let uri: hyper::Uri = format!("http://{address}/api/sessions")
            .parse()
            .expect("parse uri");
        let request = Request::builder()
            .method(hyper::Method::POST)
            .uri(uri)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "seed": 10000 + i,
                    "level": 1,
                    "opponent_type": "ai:baseline"
                })
                .to_string(),
            ))
            .expect("build request");

        let response = client.request(request).await.expect("create session");
        assert_eq!(response.status(), hyper::StatusCode::CREATED);

        let duration = start.elapsed();
        create_times.push(duration);
    }

    let avg_create_time = create_times.iter().sum::<Duration>() / create_times.len() as u32;
    println!("Average session creation time: {:?}", avg_create_time);

    // Session creation should be fast (under 100ms)
    assert!(
        avg_create_time < Duration::from_millis(100),
        "Session creation too slow: {:?}",
        avg_create_time
    );

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}

/// Test API throughput with concurrent requests
#[tokio::test]
async fn test_api_concurrent_request_throughput() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");
    let server = WebServer::from_context(context.clone());
    let handle = server.start().await.expect("start server");
    let address = handle.address();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let concurrent_requests = 100;
    let start = Instant::now();

    let mut join_set = JoinSet::new();

    for i in 0..concurrent_requests {
        let addr = address;
        join_set.spawn(async move {
            let client = HyperClient::new();
            let uri: hyper::Uri = format!("http://{addr}/api/sessions")
                .parse()
                .expect("parse uri");
            let request = Request::builder()
                .method(hyper::Method::POST)
                .uri(uri)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "seed": 20000 + i,
                        "level": 1,
                        "opponent_type": "ai:baseline"
                    })
                    .to_string(),
                ))
                .expect("build request");

            client.request(request).await.expect("request")
        });
    }

    let mut success_count = 0;
    while let Some(result) = join_set.join_next().await {
        let response = result.expect("task completed");
        if response.status() == hyper::StatusCode::CREATED {
            success_count += 1;
        }
    }

    let duration = start.elapsed();
    let requests_per_sec = (concurrent_requests as f64 / duration.as_secs_f64()) as u64;

    println!(
        "API Throughput: {} successful requests in {:?} ({} req/sec)",
        success_count, duration, requests_per_sec
    );

    assert_eq!(success_count, concurrent_requests);

    // Should handle at least 100 requests per second
    assert!(
        requests_per_sec > 100,
        "API throughput too low: {} req/sec",
        requests_per_sec
    );

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}

/// Test memory usage with many active sessions
#[tokio::test]
async fn test_memory_usage_many_sessions() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_count = 100;
    let mut session_ids = Vec::new();

    // Create many sessions
    for i in 0..session_count {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(30000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");
        session_ids.push(session_id);
    }

    // Verify all sessions are accessible
    for session_id in &session_ids {
        assert!(context.sessions().state(session_id).is_ok());
    }

    // Note: Actual memory measurement would require OS-specific APIs
    // This test ensures the system can handle many sessions without crashing
    println!(
        "Successfully created and accessed {} sessions",
        session_count
    );
}

/// Test SSE connection handling under load
#[tokio::test]
async fn test_sse_connection_handling_load() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    // Create multiple sessions, each with multiple subscribers
    let session_count = 10u64;
    let subscribers_per_session = 5;

    let mut all_subscriptions = Vec::new();

    for i in 0..session_count {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(40000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");

        for _ in 0..subscribers_per_session {
            let subscription = context.event_bus().subscribe(session_id.clone());
            all_subscriptions.push((session_id.clone(), subscription));
        }
    }

    // Broadcast events to all sessions
    let mut join_set = JoinSet::new();

    for i in 0..session_count {
        let ctx = Arc::clone(&context);
        let session_id = all_subscriptions[(i * subscribers_per_session) as usize]
            .0
            .clone();

        join_set.spawn(async move {
            for j in 0..10 {
                ctx.event_bus().broadcast(
                    &session_id,
                    GameEvent::Error {
                        session_id: session_id.clone(),
                        message: format!("load-test-{}", j),
                    },
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });
    }

    // Wait for all broadcasts
    while let Some(result) = join_set.join_next().await {
        result.expect("broadcast task");
    }

    // Verify subscribers received events
    let mut total_received = 0;
    for (_session_id, mut subscription) in all_subscriptions {
        while let Ok(Some(_)) =
            tokio::time::timeout(Duration::from_millis(10), subscription.receiver.recv()).await
        {
            total_received += 1;
        }
    }

    let expected = session_count * subscribers_per_session * 10;
    assert_eq!(
        total_received, expected,
        "All subscribers should receive events"
    );
}

/// Test graceful degradation under extreme load
#[tokio::test]
async fn test_graceful_degradation_extreme_load() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");

    let session_id = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create session");

    // Create many subscriptions
    let mut subscriptions = Vec::new();
    for _ in 0..100 {
        subscriptions.push(context.event_bus().subscribe(session_id.clone()));
    }

    // Flood with events
    let event_count = 1000;
    for i in 0..event_count {
        context.event_bus().broadcast(
            &session_id,
            GameEvent::Error {
                session_id: session_id.clone(),
                message: format!("flood-{}", i),
            },
        );
    }

    // System should not crash or deadlock
    // Some events might be dropped, but system should remain responsive
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should still be able to create new sessions
    let new_session = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create session after load");

    assert!(!new_session.is_empty());
}
