/// Concurrent session testing for race conditions and thread safety
/// Tests multiple simultaneous game sessions and concurrent operations
use axiomind_web::events::GameEvent;
use axiomind_web::server::{AppContext, ServerConfig};
use axiomind_web::session::{GameConfig, OpponentType};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinSet;

/// Test creating multiple sessions concurrently
#[tokio::test]
async fn test_concurrent_session_creation() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let mut join_set = JoinSet::new();
    let session_count: usize = 10;

    for i in 0..session_count {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            ctx.sessions()
                .create_session(GameConfig {
                    seed: Some(1000 + i as u64),
                    level: 1,
                    opponent_type: OpponentType::AI("baseline".to_string()),
                })
                .expect("create session")
        });
    }

    let mut session_ids = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let session_id = result.expect("task completed");
        session_ids.push(session_id);
    }

    // All sessions should be created with unique IDs
    assert_eq!(session_ids.len(), session_count);

    // Verify all session IDs are unique
    let unique_count = session_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(unique_count, session_count);

    // Verify all sessions are accessible
    for session_id in &session_ids {
        assert!(context.sessions().state(session_id).is_ok());
    }
}

/// Test concurrent actions on different sessions
#[tokio::test]
async fn test_concurrent_actions_different_sessions() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    // Create multiple sessions
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let session_id = context
            .sessions()
            .create_session(GameConfig {
                seed: Some(2000 + i as u64),
                level: 1,
                opponent_type: OpponentType::AI("baseline".to_string()),
            })
            .expect("create session");
        session_ids.push(session_id);
    }

    // Process actions concurrently on all sessions
    let mut join_set = JoinSet::new();

    for session_id in session_ids.clone() {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            // Take 5 actions on each session
            let mut success_count = 0;
            for _ in 0..5 {
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
                    success_count += 1;
                }

                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            success_count
        });
    }

    let mut total_actions = 0;
    while let Some(result) = join_set.join_next().await {
        let actions = result.expect("task completed");
        total_actions += actions;
    }

    // At least some actions should succeed
    assert!(total_actions > 0, "Some actions should have succeeded");
}

/// Test concurrent SSE subscriptions to same session
#[tokio::test]
async fn test_concurrent_sse_subscriptions() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(3000),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    // Create multiple SSE subscriptions concurrently
    let subscriber_count = 10;
    let mut subscriptions = Vec::new();

    for _ in 0..subscriber_count {
        let subscription = context.event_bus().subscribe(session_id.clone());
        subscriptions.push(subscription);
    }

    // Broadcast an event
    context.event_bus().broadcast(
        &session_id,
        GameEvent::Error {
            session_id: session_id.clone(),
            message: "test broadcast".to_string(),
        },
    );

    // All subscribers should receive the event
    let mut received_count = 0;
    for mut sub in subscriptions {
        if let Ok(Some(_event)) =
            tokio::time::timeout(Duration::from_millis(100), sub.receiver.recv()).await
        {
            received_count += 1;
        }
    }

    assert_eq!(
        received_count, subscriber_count,
        "All subscribers should receive the event"
    );
}

/// Test session cleanup doesn't affect other sessions
#[tokio::test]
async fn test_session_cleanup_isolation() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    // Create multiple sessions
    let session_id_1 = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(4000),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session 1");

    let session_id_2 = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(4001),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session 2");

    // Verify both sessions exist
    assert!(context.sessions().get_session(&session_id_1).is_ok());
    assert!(context.sessions().get_session(&session_id_2).is_ok());

    // Delete first session
    context
        .sessions()
        .delete_session(&session_id_1)
        .expect("delete session 1");

    // First session should be gone, second should still exist
    assert!(context.sessions().get_session(&session_id_1).is_err());
    assert!(context.sessions().get_session(&session_id_2).is_ok());

    // Second session should still be functional
    let result = context
        .sessions()
        .process_action(&session_id_2, axiomind_engine::player::PlayerAction::Check);
    assert!(result.is_ok());
}

/// Test concurrent read and write on session state
#[tokio::test]
async fn test_concurrent_session_state_access() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(5000),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    let mut join_set = JoinSet::new();

    // Spawn multiple readers
    for _ in 0..10 {
        let ctx = Arc::clone(&context);
        let sid = session_id.clone();
        join_set.spawn(async move {
            for _ in 0..20 {
                let _ = ctx.sessions().state(&sid);
                tokio::time::sleep(Duration::from_micros(100)).await;
            }
        });
    }

    // Spawn a writer that takes actions
    let ctx = Arc::clone(&context);
    let sid = session_id.clone();
    join_set.spawn(async move {
        for _ in 0..10 {
            let _ = ctx
                .sessions()
                .process_action(&sid, axiomind_engine::player::PlayerAction::Check)
                .or_else(|_| {
                    ctx.sessions()
                        .process_action(&sid, axiomind_engine::player::PlayerAction::Call)
                });
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });

    // All tasks should complete without panicking
    while let Some(result) = join_set.join_next().await {
        result.expect("task should not panic");
    }
}

/// Test event bus handles concurrent broadcasts
#[tokio::test]
async fn test_concurrent_event_broadcasts() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let session_id = context
        .sessions()
        .create_session(GameConfig {
            seed: Some(6000),
            level: 1,
            opponent_type: OpponentType::AI("baseline".to_string()),
        })
        .expect("create session");

    // Create subscriber
    let mut subscription = context.event_bus().subscribe(session_id.clone());

    // Broadcast many events concurrently
    let mut join_set = JoinSet::new();
    let event_count = 100;

    for i in 0..event_count {
        let ctx = Arc::clone(&context);
        let sid = session_id.clone();
        join_set.spawn(async move {
            ctx.event_bus().broadcast(
                &sid,
                GameEvent::Error {
                    session_id: sid.clone(),
                    message: format!("event-{}", i),
                },
            );
        });
    }

    // Wait for all broadcasts to complete
    while let Some(result) = join_set.join_next().await {
        result.expect("broadcast task completed");
    }

    // Count received events
    let mut received = 0;
    while let Ok(Some(_)) =
        tokio::time::timeout(Duration::from_millis(10), subscription.receiver.recv()).await
    {
        received += 1;
    }

    // Should receive all events (or close to it, allowing for timing)
    assert!(
        received >= event_count - 5,
        "Should receive most events: got {}/{}",
        received,
        event_count
    );
}

/// Test race condition on session creation with same parameters
#[tokio::test]
async fn test_race_condition_identical_configs() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let mut join_set = JoinSet::new();
    let create_count = 20;

    // Try to create sessions with identical config concurrently
    for _ in 0..create_count {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            ctx.sessions()
                .create_session(GameConfig {
                    seed: Some(7000), // Same seed
                    level: 1,
                    opponent_type: OpponentType::AI("baseline".to_string()),
                })
                .expect("create session")
        });
    }

    let mut session_ids = Vec::new();
    while let Some(result) = join_set.join_next().await {
        let session_id = result.expect("task completed");
        session_ids.push(session_id);
    }

    // All sessions should have unique IDs despite same config
    let unique_count = session_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(
        unique_count, create_count,
        "All sessions should have unique IDs"
    );
}

/// Test deadlock prevention with complex concurrent operations
#[tokio::test]
async fn test_no_deadlock_complex_operations() {
    let context = Arc::new(AppContext::new(ServerConfig::for_tests()).expect("create context"));

    let mut join_set = JoinSet::new();

    // Task 1: Create sessions
    for i in 0..5 {
        let ctx = Arc::clone(&context);
        join_set.spawn(async move {
            ctx.sessions()
                .create_session(GameConfig {
                    seed: Some(8000 + i as u64),
                    level: 1,
                    opponent_type: OpponentType::AI("baseline".to_string()),
                })
                .expect("create session")
        });
    }

    // Collect created session IDs
    let mut session_ids = Vec::new();
    while let Some(result) = join_set.join_next().await {
        session_ids.push(result.expect("create task"));
    }

    // Task 2: Mix operations on all sessions
    let mut task_set = JoinSet::new();
    for session_id in session_ids.clone() {
        let ctx = Arc::clone(&context);
        let sid = session_id.clone();
        task_set.spawn(async move {
            // Subscribe to events
            let _sub = ctx.event_bus().subscribe(sid.clone());

            // Get state
            let _ = ctx.sessions().state(&sid);

            // Process action
            let _ = ctx
                .sessions()
                .process_action(&sid, axiomind_engine::player::PlayerAction::Check);

            sid // Return session ID
        });
    }

    // Should complete without deadlock within reasonable time
    let timeout_result = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(result) = task_set.join_next().await {
            result.expect("operation task");
        }
    })
    .await;

    assert!(
        timeout_result.is_ok(),
        "Operations should complete without deadlock"
    );
}
