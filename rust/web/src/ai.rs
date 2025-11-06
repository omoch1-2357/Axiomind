mod baseline;

pub use baseline::BaselineAI;

use axm_engine::engine::Engine;
use axm_engine::player::PlayerAction;

/// Trait for AI opponents that can make decisions in poker games
///
/// This trait allows for pluggable AI strategies that can be used
/// to generate actions based on the current game state.
pub trait AIOpponent: Send + Sync {
    /// Get the next action for the AI player based on current game engine state
    ///
    /// # Arguments
    /// * `engine` - Reference to the game engine with current state
    /// * `player_id` - The ID of the AI player (0 or 1)
    ///
    /// # Returns
    /// The action the AI decides to take
    fn get_action(&self, engine: &Engine, player_id: usize) -> PlayerAction;

    /// Get the name/identifier of this AI strategy
    fn name(&self) -> &str;
}

/// Factory function to create AI opponents by name
pub fn create_ai(name: &str) -> Box<dyn AIOpponent> {
    match name {
        "baseline" | "" => Box::new(BaselineAI::new()),
        custom => Box::new(BaselineAI::with_name(custom.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axm_engine::engine::Engine;
    use axm_engine::player::PlayerAction;

    struct MockAI {
        name: String,
        next_action: PlayerAction,
    }

    impl AIOpponent for MockAI {
        fn get_action(&self, _engine: &Engine, _player_id: usize) -> PlayerAction {
            self.next_action.clone()
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[test]
    fn ai_opponent_trait_provides_action() {
        let ai = MockAI {
            name: "test_ai".to_string(),
            next_action: PlayerAction::Check,
        };

        let engine = Engine::new(Some(42), 1);

        let action = ai.get_action(&engine, 1);
        assert_eq!(action, PlayerAction::Check);
        assert_eq!(ai.name(), "test_ai");
    }

    #[test]
    fn ai_opponent_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn AIOpponent>>();
    }

    #[test]
    fn create_ai_returns_baseline_for_baseline_name() {
        let ai = create_ai("baseline");
        assert_eq!(ai.name(), "baseline");
    }

    #[test]
    fn create_ai_returns_baseline_for_empty_name() {
        let ai = create_ai("");
        assert_eq!(ai.name(), "baseline");
    }

    #[test]
    fn create_ai_returns_custom_name_for_unknown_strategy() {
        let ai = create_ai("custom_strategy");
        assert_eq!(ai.name(), "custom_strategy");
    }

    #[test]
    fn created_ai_can_provide_actions() {
        let ai = create_ai("baseline");
        let engine = Engine::new(Some(42), 1);

        let action = ai.get_action(&engine, 1);
        // Should return some valid action
        assert!(matches!(
            action,
            PlayerAction::Check
                | PlayerAction::Fold
                | PlayerAction::Call
                | PlayerAction::Bet(_)
                | PlayerAction::Raise(_)
                | PlayerAction::AllIn
        ));
    }
}
