pub mod events;
pub mod handlers;
pub mod server;
pub mod session;
pub mod static_handler;

pub use events::{EventBus, GameEvent, PlayerInfo};
pub use server::{AppContext, ServerConfig, ServerError, ServerHandle, WebServer};
pub use session::{
    AvailableAction, GameConfig, GameSessionState, GameStateResponse, OpponentType,
    PlayerStateResponse, SeatPosition, SessionError, SessionId, SessionManager,
};
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
