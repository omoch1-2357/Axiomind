pub mod ai;
pub mod errors;
pub mod events;
pub mod handlers;
pub mod history;
pub mod logging;
pub mod metrics;
pub mod middleware;
pub mod server;
pub mod session;
pub mod settings;
pub mod static_handler;

pub use ai::{AIOpponent, BaselineAI, create_ai};
pub use errors::{ErrorResponse, ErrorSeverity, IntoErrorResponse};
pub use events::{EventBus, GameEvent, PlayerInfo};
pub use history::{HandFilter, HandStatistics, HistoryError, HistoryStore};
pub use logging::{LogEntry, TestLogSubscriber, init_logging, init_test_logging};
pub use metrics::{MetricsCollector, MetricsSnapshot, RequestTimer};
pub use middleware::{RequestMetrics, log_response, with_request_logging};
pub use server::{AppContext, ServerConfig, ServerError, ServerHandle, WebServer};
pub use session::{
    AvailableAction, GameConfig, GameSessionState, GameStateResponse, OpponentType,
    PlayerStateResponse, SeatPosition, SessionError, SessionId, SessionManager,
};
pub use settings::{AppSettings, SettingsError, SettingsStore};
pub use static_handler::{StaticError, StaticHandler};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_provides_shared_components() {
        let ctx = AppContext::new_for_tests();

        let event_bus = ctx.event_bus();
        let sessions = ctx.sessions();

        assert_eq!(event_bus.subscriber_count(), 0);
        assert!(sessions.active_sessions().is_empty());
    }
}
