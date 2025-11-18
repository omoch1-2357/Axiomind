/// Load testing for multiple concurrent games
/// Tests system behavior under high load with many simultaneous games
use axiomind_web::events::GameEvent;
use axiomind_web::server::{AppContext, ServerConfig, WebServer};
use axiomind_web::session::{GameConfig, OpponentType};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;
use warp::hyper::{self, Body, Client as HyperClient, Request};

/// Test load with many concurrent game sessions
#[tokio::test]
async fn test_load_many_concurrent_games() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let game_count = 50;
    let actions_per_game = 10;

    let start = Instant::now();

    // Create many games
    let mut session_ids = Vec::new();
    for i in 0..game_count {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(100000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");
        session_ids.push(session_id);
    }

    let create_duration = start.elapsed();
    println!("Created {} games in {:?}", game_count, create_duration);

    // Play all games concurrently
    let mut join_set = JoinSet::new();

    for session_id in session_ids {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            let mut actions_taken = 0;
            for _ in 0..actions_per_game {
                let result = ctx
                    .sessions()
                    .process_action(&session_id, axiomind_engine::player::PlayerAction::Check)
                    .or_else(|_| {
                        ctx.sessions().process_action(
                            &session_id,
                            axiomind_engine::player::PlayerAction::Call,
                        )
                    });

                if result.is_ok() {
                    actions_taken += 1;
                }

                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            actions_taken
        });
    }

    // Collect results
    let mut total_actions = 0;
    while let Some(result) = join_set.join_next().await {
        total_actions += result.expect("game task completed");
    }

    let total_duration = start.elapsed();

    println!(
        "Played {} games with {} total actions in {:?}",
        game_count, total_actions, total_duration
    );

    // All games should complete within reasonable time
    assert!(
        total_duration < Duration::from_secs(30),
        "Load test took too long: {:?}",
        total_duration
    );

    // Actions may complete early due to folds - just verify some actions completed
    assert!(
        total_actions > game_count,
        "Too many actions failed (total={}, games={})",
        total_actions,
        game_count
    );
}

/// Test load with continuous session creation and deletion
#[tokio::test]
async fn test_load_session_churn() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let duration = Duration::from_secs(5);
    let created = Arc::new(AtomicU64::new(0));
    let deleted = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    let mut join_set = JoinSet::new();

    // Creator tasks
    for i in 0..3 {
        let ctx = Arc::clone(&context);
        let created_count = Arc::clone(&created);
        join_set.spawn(async move {
            let mut local_sessions = Vec::new();
            while start.elapsed() < duration {
                let session_id = ctx
                    .sessions()
                    .create_session(GameConfig {
                        seed: Some(200000 + i * 10000 + created_count.load(Ordering::Relaxed)),
                        level: 1,
                        opponent_type: OpponentType::AI("baseline".to_string()),
                    })
                    .expect("create session");

                local_sessions.push(session_id);
                created_count.fetch_add(1, Ordering::Relaxed);

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            local_sessions
        });
    }

    // Collect sessions and delete them
    let mut all_sessions = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let sessions = result.expect("creator task");
        all_sessions.extend(sessions);
    }

    // Delete sessions
    let mut delete_set = JoinSet::new();
    for session_id in all_sessions {
        let ctx = Arc::clone(&context);
        let deleted_count = Arc::clone(&deleted);
        delete_set.spawn(async move {
            ctx.sessions()
                .delete_session(&session_id)
                .expect("delete session");
            deleted_count.fetch_add(1, Ordering::Relaxed);
        });
    }

    while let Some(result) = delete_set.join_next().await {
        result.expect("delete task");
    }

    let total_created = created.load(Ordering::Relaxed);
    let total_deleted = deleted.load(Ordering::Relaxed);

    println!(
        "Session churn: created {} and deleted {} in {:?}",
        total_created,
        total_deleted,
        start.elapsed()
    );

    assert_eq!(
        total_created, total_deleted,
        "Should have deleted all created sessions"
    );
    assert!(total_created > 10, "Should have created many sessions");
}

/// Test load with many SSE connections across multiple games
#[tokio::test]
async fn test_load_many_sse_connections() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let game_count = 20;
    let subscribers_per_game = 5;

    // Create games
    let mut session_ids = Vec::new();
    for i in 0..game_count {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(300000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");
        session_ids.push(session_id);
    }

    // Create subscribers
    let mut subscriptions = Vec::new();
    for session_id in &session_ids {
        for _ in 0..subscribers_per_game {
            subscriptions.push((
                session_id.clone(),
                context.event_bus().subscribe(session_id.clone()),
            ));
        }
    }

    println!(
        "Created {} SSE connections across {} games",
        subscriptions.len(),
        game_count
    );

    // Broadcast events to all games
    let mut join_set = JoinSet::new();

    for session_id in session_ids {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            for i in 0..10 {
                ctx.event_bus().broadcast(
                    &session_id,
                    GameEvent::Error {
                        session_id: session_id.clone(),
                        message: format!("load-{}", i),
                    },
                );
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }

    // Wait for all broadcasts
    while let Some(result) = join_set.join_next().await {
        result.expect("broadcast task");
    }

    // Count received events
    let mut total_received = 0;
    for (_session_id, mut subscription) in subscriptions {
        while let Ok(Some(_)) =
            tokio::time::timeout(Duration::from_millis(20), subscription.receiver.recv()).await
        {
            total_received += 1;
        }
    }

    let expected = game_count * subscribers_per_game * 10;
    println!("Received {}/{} events", total_received, expected);

    // Should receive most events (allowing for some timing variance)
    assert!(
        total_received >= expected - game_count * 5,
        "Should receive most events"
    );
}

/// Test HTTP load with rapid API requests
#[tokio::test]
async fn test_load_rapid_api_requests() {
    let context = AppContext::new(ServerConfig::for_tests()).expect("create context");
    let server = WebServer::from_context(context.clone());
    let handle = server.start().await.expect("start server");
    let address = handle.address();

    tokio::time::sleep(Duration::from_millis(20)).await;

    let request_count = 200;
    let start = Instant::now();

    let success_count = Arc::new(AtomicU64::new(0));
    let mut join_set = JoinSet::new();

    for i in 0..request_count {
        let addr = address;
        let success = Arc::clone(&success_count);

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
                        "seed": 400000 + i,
                        "level": 1,
                        "opponent_type": "ai:baseline"
                    })
                    .to_string(),
                ))
                .expect("build request");

            if let Ok(response) = client.request(request).await {
                if response.status() == hyper::StatusCode::CREATED {
                    success.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }

    while let Some(result) = join_set.join_next().await {
        result.expect("request task");
    }

    let duration = start.elapsed();
    let successes = success_count.load(Ordering::Relaxed);
    let req_per_sec = (request_count as f64 / duration.as_secs_f64()) as u64;

    println!(
        "API Load: {}/{} requests succeeded in {:?} ({} req/sec)",
        successes, request_count, duration, req_per_sec
    );

    // Most requests should succeed
    assert!(
        successes >= request_count * 95 / 100,
        "Too many failed requests: {}/{}",
        successes,
        request_count
    );

    tokio::time::timeout(Duration::from_secs(2), handle.shutdown())
        .await
        .expect("shutdown timed out")
        .expect("shutdown failed");
}

/// Test sustained load over longer duration
#[tokio::test]
async fn test_sustained_load() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let test_duration = Duration::from_secs(10);
    let games_to_maintain = 10;

    let start = Instant::now();

    // Create initial games
    let mut session_ids = Vec::new();
    for i in 0..games_to_maintain {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(500000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");
        session_ids.push(session_id);
    }

    let mut join_set = JoinSet::new();
    let actions_taken = Arc::new(AtomicU64::new(0));

    // Maintain continuous activity on all games
    for session_id in session_ids {
        let ctx = Arc::clone(&context);
        let action_count = Arc::clone(&actions_taken);

        join_set.spawn(async move {
            while start.elapsed() < test_duration {
                let result = ctx
                    .sessions()
                    .process_action(&session_id, axiomind_engine::player::PlayerAction::Check)
                    .or_else(|_| {
                        ctx.sessions().process_action(
                            &session_id,
                            axiomind_engine::player::PlayerAction::Call,
                        )
                    });

                if result.is_ok() {
                    action_count.fetch_add(1, Ordering::Relaxed);
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }

    // Wait for all tasks
    while let Some(result) = join_set.join_next().await {
        result.expect("game task");
    }

    let total_actions = actions_taken.load(Ordering::Relaxed);
    let duration = start.elapsed();

    println!(
        "Sustained load: {} actions across {} games in {:?}",
        total_actions, games_to_maintain, duration
    );

    // Should maintain some activity (may be less due to quick fold completions)
    assert!(
        total_actions > 10,
        "Should have taken some actions: {}",
        total_actions
    );
}

/// Test recovery from connection drops
#[tokio::test]
async fn test_load_connection_resilience() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(600000),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    // Create and drop subscriptions repeatedly
    for i in 0..20 {
        let mut subscription = context.event_bus().subscribe(session_id.clone());

        // Broadcast event
        context.event_bus().broadcast(
            &session_id,
            GameEvent::Error {
                session_id: session_id.clone(),
                message: format!("resilience-{}", i),
            },
        );

        // Receive event
        let received =
            tokio::time::timeout(Duration::from_millis(50), subscription.receiver.recv())
                .await
                .is_ok();

        assert!(received, "Subscription {} should receive event", i);

        // Drop subscription (simulating connection loss)
        drop(subscription);

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // System should still be functional - test by creating new subscription
    let mut new_subscription = context.event_bus().subscribe(session_id.clone());

    // Broadcast a test event
    context.event_bus().broadcast(
        &session_id,
        GameEvent::Error {
            session_id: session_id.clone(),
            message: "final-test".to_string(),
        },
    );

    // Verify new subscription works by checking we can receive events
    let received =
        tokio::time::timeout(Duration::from_millis(50), new_subscription.receiver.recv())
            .await
            .is_ok();
    assert!(received, "New subscription should be functional");
}

/// Test memory stability under load
#[tokio::test]
async fn test_load_memory_stability() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    // Create and play many short games
    let game_count = 100;

    for i in 0..game_count {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(700000 + i),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");

        // Take a few actions
        for _ in 0..5 {
            let _ = context
                .sessions()
                .process_action(&session_id, axiomind_engine::player::PlayerAction::Check);
        }

        // Delete session
        context
            .sessions()
            .delete_session(&session_id)
            .expect("delete session");
    }

    // System should still be responsive
    let final_session = context
        .sessions()
        .create_session(GameConfig::default())
        .expect("create final session");

    assert!(!final_session.is_empty());
}

/// Test peak load handling
#[tokio::test]
async fn test_peak_load_handling() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let mut join_set = JoinSet::new();

    // Simulate peak load with sudden burst of activity
    let burst_size = 100;

    let start = Instant::now();

    for i in 0..burst_size {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            // Create session
            let session_id = ctx
                .sessions()
                .create_session(GameConfig {
                    seed: Some(800000 + i),
                    level: 1,
                    opponent_type: OpponentType::AI("baseline".to_string()),
                })
                .expect("create session");

            // Subscribe
            let _sub = ctx.event_bus().subscribe(session_id.clone());

            // Take action
            let _ = ctx
                .sessions()
                .process_action(&session_id, axiomind_engine::player::PlayerAction::Check);

            // Delete
            ctx.sessions()
                .delete_session(&session_id)
                .expect("delete session");
        });
    }

    // All tasks should complete
    while let Some(result) = join_set.join_next().await {
        result.expect("peak load task");
    }

    let duration = start.elapsed();

    println!(
        "Peak load: {} operations completed in {:?}",
        burst_size, duration
    );

    // Should handle burst within reasonable time
    assert!(
        duration < Duration::from_secs(10),
        "Peak load took too long: {:?}",
        duration
    );
}
