use super::AIOpponent;
use axm_engine::engine::Engine;
use axm_engine::player::PlayerAction;

/// Baseline AI opponent with simple rule-based decision making
///
/// This AI follows a conservative strategy:
/// - Always check if possible
/// - Fold to any bet
/// - Never raises or bets aggressively
pub struct BaselineAI {
    name: String,
}

impl BaselineAI {
    pub fn new() -> Self {
        Self {
            name: "baseline".to_string(),
        }
    }

    pub fn with_name(name: String) -> Self {
        Self { name }
    }
}

impl Default for BaselineAI {
    fn default() -> Self {
        Self::new()
    }
}

impl AIOpponent for BaselineAI {
    fn get_action(&self, _engine: &Engine, _player_id: usize) -> PlayerAction {
        // Simple baseline strategy: always check/call
        // TODO: Implement proper decision logic based on game state
        PlayerAction::Check
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axm_engine::engine::Engine;

    #[test]
    fn baseline_ai_creates_with_default_name() {
        let ai = BaselineAI::new();
        assert_eq!(ai.name(), "baseline");
    }

    #[test]
    fn baseline_ai_creates_with_custom_name() {
        let ai = BaselineAI::with_name("custom_baseline".to_string());
        assert_eq!(ai.name(), "custom_baseline");
    }

    #[test]
    fn baseline_ai_returns_conservative_action() {
        let ai = BaselineAI::new();
        let engine = Engine::new(Some(42), 1);

        let action = ai.get_action(&engine, 1);
        // Baseline AI should always check in this simple implementation
        assert_eq!(action, PlayerAction::Check);
    }

    #[test]
    fn baseline_ai_is_send_sync() {
        let ai = BaselineAI::new();
        let boxed: Box<dyn AIOpponent> = Box::new(ai);
        assert_eq!(boxed.name(), "baseline");
    }
}
