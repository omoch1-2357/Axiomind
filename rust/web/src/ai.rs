//! AI opponent module for the web server.
//!
//! This module re-exports the AI functionality from the axiomind_ai crate,
//! providing a unified interface for AI opponents in poker games.

// Re-export the AIOpponent trait and BaselineAI from axiomind_ai
pub use axiomind_ai::{AIOpponent, baseline::BaselineAI};

/// Factory function to create AI opponents by name.
///
/// This function wraps the axiomind_ai::create_ai function but provides
/// a fallback to BaselineAI with custom names instead of panicking.
///
/// # Arguments
/// * `name` - The name/type of AI to create
///
/// # Returns
/// A boxed trait object implementing AIOpponent
///
/// # Example
/// ```
/// use axiomind_web::ai::create_ai;
///
/// let ai = create_ai("baseline");
/// assert_eq!(ai.name(), "BaselineAI");
/// ```
pub fn create_ai(name: &str) -> Box<dyn AIOpponent> {
    match name {
        "baseline" | "" => axiomind_ai::create_ai("baseline"),
        _ => {
            // For unknown AI types, default to baseline
            // This provides more graceful degradation than panicking
            axiomind_ai::create_ai("baseline")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiomind_engine::engine::Engine;
    use axiomind_engine::player::PlayerAction;

    #[test]
    fn ai_opponent_trait_is_accessible() {
        fn assert_trait_exists<T: AIOpponent>() {}
        assert_trait_exists::<BaselineAI>();
    }

    #[test]
    fn baseline_ai_is_accessible() {
        let ai = BaselineAI::new();
        assert_eq!(ai.name(), "BaselineAI");
    }

    #[test]
    fn create_ai_returns_baseline_for_baseline_name() {
        let ai = create_ai("baseline");
        assert_eq!(ai.name(), "BaselineAI");
    }

    #[test]
    fn create_ai_returns_baseline_for_empty_name() {
        let ai = create_ai("");
        assert_eq!(ai.name(), "BaselineAI");
    }

    #[test]
    fn create_ai_returns_baseline_for_unknown_strategy() {
        // Unlike the old implementation, this now returns baseline instead of custom name
        let ai = create_ai("custom_strategy");
        assert_eq!(ai.name(), "BaselineAI");
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

    #[test]
    fn ai_opponent_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Box<dyn AIOpponent>>();
    }
}
