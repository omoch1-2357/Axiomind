use crate::errors::IntoErrorResponse;
use crate::history::{HandFilter, HandStatistics};
use crate::server::AppContext;
use axiomind_engine::logger::HandRecord;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use warp::http::StatusCode;
use warp::{reject, reply, Filter, Rejection, Reply};

/// GET /api/history?limit=N
/// Get recent hands with optional limit
pub fn get_recent_hands(
    ctx: Arc<AppContext>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "history")
        .and(warp::get())
        .and(warp::query::<GetHistoryQuery>())
        .and(with_context(ctx))
        .and_then(handle_get_recent_hands)
}

/// GET /api/history/:hand_id
/// Get a specific hand by ID
pub fn get_hand_by_id(
    ctx: Arc<AppContext>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "history" / String)
        .and(warp::get())
        .and(with_context(ctx))
        .and_then(handle_get_hand_by_id)
}

/// POST /api/history/filter
/// Filter hands by criteria
pub fn filter_hands(
    ctx: Arc<AppContext>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "history" / "filter")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(ctx))
        .and_then(handle_filter_hands)
}

/// GET /api/history/stats
/// Get statistics from hand history
pub fn get_statistics(
    ctx: Arc<AppContext>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("api" / "history" / "stats")
        .and(warp::get())
        .and(with_context(ctx))
        .and_then(handle_get_statistics)
}

fn with_context(
    ctx: Arc<AppContext>,
) -> impl Filter<Extract = (Arc<AppContext>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || Arc::clone(&ctx))
}

#[derive(Debug, Deserialize)]
pub struct GetHistoryQuery {
    #[serde(default)]
    pub limit: Option<usize>,
}

async fn handle_get_recent_hands(
    query: GetHistoryQuery,
    ctx: Arc<AppContext>,
) -> Result<impl Reply, Rejection> {
    let history = ctx.history();
    let response = match history.get_recent_hands(query.limit) {
        Ok(hands) => reply::json(&hands).into_response(),
        Err(err) => err.into_http_response(),
    };

    Ok::<_, Rejection>(response)
}

async fn handle_get_hand_by_id(
    hand_id: String,
    ctx: Arc<AppContext>,
) -> Result<impl Reply, Rejection> {
    let history = ctx.history();
    let response = match history.get_hand(&hand_id) {
        Ok(Some(hand)) => reply::with_status(reply::json(&hand), StatusCode::OK).into_response(),
        Ok(None) => return Err(reject::not_found()),
        Err(err) => err.into_http_response(),
    };

    Ok::<_, Rejection>(response)
}

async fn handle_filter_hands(
    filter: HandFilter,
    ctx: Arc<AppContext>,
) -> Result<impl Reply, Rejection> {
    let history = ctx.history();
    let response = match history.filter_hands(filter) {
        Ok(hands) => reply::json(&hands).into_response(),
        Err(err) => err.into_http_response(),
    };

    Ok::<_, Rejection>(response)
}

async fn handle_get_statistics(ctx: Arc<AppContext>) -> Result<impl Reply, Rejection> {
    let history = ctx.history();
    let response = match history.calculate_stats() {
        Ok(stats) => reply::json(&stats).into_response(),
        Err(err) => err.into_http_response(),
    };

    Ok::<_, Rejection>(response)
}

/// Response for history list
#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryListResponse {
    pub hands: Vec<HandRecord>,
    pub total: usize,
}

/// Response for statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub stats: HandStatistics,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiomind_engine::cards::{Card, Rank, Suit};
    use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
    use axiomind_engine::player::PlayerAction;

    fn create_test_context_with_history() -> Arc<AppContext> {
        let ctx = AppContext::new_for_tests();
        let history = ctx.history();

        // Add test hands
        for i in 0..5 {
            let hand = HandRecord {
                hand_id: format!("test-{:03}", i),
                seed: Some(42),
                actions: vec![ActionRecord {
                    player_id: 0,
                    street: Street::Preflop,
                    action: PlayerAction::Check,
                }],
                board: vec![Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                }],
                result: Some("player 0 wins".to_string()),
                ts: Some(format!("2025-01-0{}T10:00:00Z", i + 1)),
                meta: None,
                showdown: None,
            };
            history.add_hand(hand).expect("add hand");
        }

        Arc::new(ctx)
    }

    #[tokio::test]
    async fn test_get_recent_hands_endpoint() {
        let ctx = create_test_context_with_history();
        let filter = get_recent_hands(ctx);

        let response = warp::test::request()
            .method("GET")
            .path("/api/history?limit=3")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let hands: Vec<HandRecord> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(hands.len(), 3);
    }

    #[tokio::test]
    async fn test_get_hand_by_id_endpoint() {
        let ctx = create_test_context_with_history();
        let filter = get_hand_by_id(ctx);

        let response = warp::test::request()
            .method("GET")
            .path("/api/history/test-001")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let hand: HandRecord = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(hand.hand_id, "test-001");
    }

    #[tokio::test]
    async fn test_get_nonexistent_hand_returns_404() {
        let ctx = create_test_context_with_history();
        let filter = get_hand_by_id(ctx);

        let response = warp::test::request()
            .method("GET")
            .path("/api/history/nonexistent")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_filter_hands_endpoint() {
        let ctx = create_test_context_with_history();
        let filter = filter_hands(ctx);

        let filter_body = HandFilter {
            result_type: Some("player 0 wins".to_string()),
            date_from: None,
            date_to: None,
        };

        let response = warp::test::request()
            .method("POST")
            .path("/api/history/filter")
            .json(&filter_body)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let hands: Vec<HandRecord> = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(hands.len(), 5);
    }

    #[tokio::test]
    async fn test_get_statistics_endpoint() {
        let ctx = create_test_context_with_history();
        let filter = get_statistics(ctx);

        let response = warp::test::request()
            .method("GET")
            .path("/api/history/stats")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), StatusCode::OK);

        let stats: HandStatistics = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(stats.total_hands, 5);
        assert_eq!(stats.wins, 5);
        assert_eq!(stats.win_rate, 100.0);
    }
}
