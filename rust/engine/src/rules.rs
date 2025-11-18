use crate::errors::GameError;
use crate::player::PlayerAction as A;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedAction {
    Fold,
    Check,
    Call(u32),
    Bet(u32),
    Raise(u32),
    AllIn(u32),
}

/// Validates a player action according to betting rules and stack size.
///
/// Converts a [`crate::player::PlayerAction`] into a [`ValidatedAction`], enforcing poker
/// betting rules, minimum raise amounts, and all-in logic when the player
/// doesn't have enough chips.
///
/// # Arguments
///
/// * `stack` - Player's remaining chip stack
/// * `to_call` - Amount needed to call the current bet
/// * `min_raise` - Minimum allowed raise amount (typically previous raise size)
/// * `action` - The action the player wishes to perform
///
/// # Returns
///
/// Returns `Ok(ValidatedAction)` if the action is legal, containing the
/// actual action to execute (which may be converted to AllIn if stack is insufficient).
///
/// # Errors
///
/// Returns [`GameError`] in the following cases:
/// - [`GameError::InsufficientChips`] - Player tries to check when facing a bet
/// - [`GameError::InvalidBetAmount`] - Bet/raise amount is below minimum or zero
///
/// # Examples
///
/// ```
/// use axiomind_engine::rules::{validate_action, ValidatedAction};
/// use axiomind_engine::player::PlayerAction;
///
/// // Valid call with sufficient stack
/// let result = validate_action(1000, 50, 100, PlayerAction::Call);
/// assert!(matches!(result, Ok(ValidatedAction::Call(50))));
///
/// // All-in when stack is insufficient for full raise
/// let result = validate_action(80, 50, 100, PlayerAction::Raise(100));
/// assert!(matches!(result, Ok(ValidatedAction::AllIn(80))));
/// ```
///
/// ```
/// use axiomind_engine::rules::validate_action;
/// use axiomind_engine::player::PlayerAction;
/// use axiomind_engine::errors::GameError;
///
/// // Invalid: check when facing a bet
/// let result = validate_action(1000, 50, 100, PlayerAction::Check);
/// assert!(matches!(result, Err(GameError::InsufficientChips)));
///
/// // Invalid: raise below minimum
/// let result = validate_action(1000, 50, 100, PlayerAction::Raise(50));
/// assert!(matches!(result, Err(GameError::InvalidBetAmount { .. })));
/// ```
pub fn validate_action(
    stack: u32,
    to_call: u32,
    min_raise: u32,
    action: A,
) -> Result<ValidatedAction, GameError> {
    match action {
        A::Fold => Ok(ValidatedAction::Fold),
        A::Check => {
            if to_call == 0 {
                Ok(ValidatedAction::Check)
            } else {
                Err(GameError::InsufficientChips)
            }
        }
        A::Call => {
            if stack <= to_call {
                Ok(ValidatedAction::AllIn(stack))
            } else {
                Ok(ValidatedAction::Call(to_call))
            }
        }
        A::Bet(amount) => {
            if amount == 0 {
                return Err(GameError::InvalidBetAmount { amount, minimum: 1 });
            }
            if amount >= stack {
                Ok(ValidatedAction::AllIn(stack))
            } else {
                Ok(ValidatedAction::Bet(amount))
            }
        }
        A::Raise(amount) => {
            if amount + to_call >= stack {
                Ok(ValidatedAction::AllIn(stack))
            } else if amount < min_raise {
                Err(GameError::InvalidBetAmount {
                    amount,
                    minimum: min_raise,
                })
            } else {
                Ok(ValidatedAction::Raise(amount))
            }
        }
        A::AllIn => Ok(ValidatedAction::AllIn(stack)),
    }
}
