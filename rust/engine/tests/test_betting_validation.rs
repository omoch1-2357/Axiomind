use axiomind_engine::errors::GameError;
use axiomind_engine::player::PlayerAction as A;
use axiomind_engine::rules::{ValidatedAction, validate_action};

#[test]
fn bet_zero_is_invalid() {
    let err = validate_action(
        10_000,
        /*to_call*/ 0,
        /*min_raise*/ 100,
        A::Bet(0),
    )
    .unwrap_err();
    match err {
        GameError::InvalidBetAmount { .. } => {}
        _ => panic!("expected InvalidBetAmount"),
    }
}

#[test]
fn bet_over_stack_becomes_allin() {
    let va = validate_action(50, 0, 100, A::Bet(100)).unwrap();
    assert_eq!(va, ValidatedAction::AllIn(50));
}

#[test]
fn call_with_insufficient_stack_is_allin_call() {
    let va = validate_action(60, 100, 100, A::Call).unwrap();
    assert_eq!(va, ValidatedAction::AllIn(60));
}

#[test]
fn short_raise_becomes_allin_without_error() {
    // to_call=100, min_raise=100, stack=130, Raise(50) -> AllIn(130)
    let va = validate_action(130, 100, 100, A::Raise(50)).unwrap();
    assert_eq!(va, ValidatedAction::AllIn(130));
}
