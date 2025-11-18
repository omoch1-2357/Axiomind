use axiomind_engine::cards::{Card, Rank, Suit};
use axiomind_engine::game::GameState;
use axiomind_engine::player::{Player, PlayerAction, Position, STARTING_STACK};

#[test]
fn players_start_with_20000_and_positions() {
    let p1 = Player::new(0, STARTING_STACK, Position::Button);
    let p2 = Player::new(1, STARTING_STACK, Position::BigBlind);
    assert_eq!(p1.stack(), 20_000);
    assert_eq!(p2.stack(), 20_000);
    assert_eq!(p1.position(), Position::Button);
    assert_eq!(p2.position(), Position::BigBlind);
}

#[test]
fn player_receives_two_hole_cards() {
    let mut p = Player::new(0, STARTING_STACK, Position::Button);
    let a = Card {
        suit: Suit::Spades,
        rank: Rank::Ace,
    };
    let k = Card {
        suit: Suit::Spades,
        rank: Rank::King,
    };
    p.give_card(a).unwrap();
    p.give_card(k).unwrap();
    let hc = p.hole_cards();
    assert_eq!(hc[0], Some(a));
    assert_eq!(hc[1], Some(k));
}

#[test]
fn betting_reduces_stack_and_cannot_overbet() {
    let mut p = Player::new(0, STARTING_STACK, Position::Button);
    p.bet(500).expect("bet should succeed");
    assert_eq!(p.stack(), 19_500);
    let err = p.bet(100_000).unwrap_err();
    assert!(err.contains("Insufficient"));
}

#[test]
fn game_state_rotates_button() {
    let p1 = Player::new(0, STARTING_STACK, Position::Button);
    let p2 = Player::new(1, STARTING_STACK, Position::BigBlind);
    let mut gs = GameState::new([p1, p2], 1);
    assert_eq!(gs.button_index(), 0);
    assert_eq!(gs.players()[0].position(), Position::Button);
    assert_eq!(gs.players()[1].position(), Position::BigBlind);
    gs.rotate_button();
    assert_eq!(gs.button_index(), 1);
    assert_eq!(gs.players()[0].position(), Position::BigBlind);
    assert_eq!(gs.players()[1].position(), Position::Button);
}

#[test]
fn player_action_enum_is_available() {
    let a = PlayerAction::Bet(123);
    match a {
        PlayerAction::Bet(n) => assert_eq!(n, 123),
        _ => panic!("expected Bet variant"),
    }
}
