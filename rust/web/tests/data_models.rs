use axiomind_engine::cards::{Card, Rank, Suit};
use axiomind_engine::logger::Street;
use axiomind_engine::player::{PlayerAction, Position as EnginePosition};
use axiomind_web::events::PlayerInfo;
use axiomind_web::session::{
    AvailableAction, GameConfig, GameSessionState, GameStateResponse, OpponentType,
    PlayerStateResponse, SeatPosition,
};
use axiomind_web::GameEvent;

#[test]
fn game_config_defaults_match_spec() {
    let cfg = GameConfig::default();
    assert_eq!(cfg.seed, None);
    assert_eq!(cfg.level, 1);
    assert_eq!(cfg.opponent_type, OpponentType::AI("baseline".into()));

    let json = serde_json::to_value(&cfg).expect("serialize config");
    assert_eq!(json["level"], 1);
    assert_eq!(json["opponent_type"], "ai:baseline");
}

#[test]
fn game_event_serializes_with_type_tag() {
    let event = GameEvent::HandStarted {
        session_id: "s1".into(),
        hand_id: "h1".into(),
        button_player: 0,
    };

    let value = serde_json::to_value(&event).expect("serialize event");
    assert_eq!(value["type"], "hand_started");
    assert_eq!(value["session_id"], "s1");
    assert_eq!(value["hand_id"], "h1");
}

#[test]
fn game_state_response_roundtrips_via_json() {
    let players = vec![PlayerStateResponse {
        id: 0,
        stack: 20_000,
        position: SeatPosition::from(EnginePosition::Button),
        hole_cards: Some(vec![
            card(Suit::Spades, Rank::Ace),
            card(Suit::Spades, Rank::King),
        ]),
        is_active: true,
        last_action: Some(PlayerAction::Raise(500)),
    }];

    let state = GameStateResponse {
        session_id: "s1".into(),
        players,
        board: vec![card(Suit::Hearts, Rank::Ten)],
        pot: 750,
        current_player: Some(0),
        available_actions: vec![AvailableAction {
            action_type: "raise".into(),
            min_amount: Some(500),
            max_amount: Some(2000),
        }],
        hand_id: Some("h1".into()),
        street: Some(Street::Flop),
    };

    let json = serde_json::to_string(&state).expect("serialize state");
    let decoded: GameStateResponse = serde_json::from_str(&json).expect("deserialize state");
    assert_eq!(decoded.session_id, "s1");
    assert_eq!(decoded.players.len(), 1);
    assert_eq!(decoded.available_actions[0].action_type, "raise");
}

#[test]
fn game_session_state_serializes_enum_variants() {
    let state = GameSessionState::HandInProgress {
        hand_id: "h1".into(),
        current_player: 1,
        street: Street::Turn,
    };

    let value = serde_json::to_value(&state).expect("serialize state");
    assert_eq!(value["status"], "hand_in_progress");
    assert_eq!(value["hand_id"], "h1");
    assert_eq!(value["current_player"], 1);
    assert_eq!(value["street"], "Turn");
}

#[test]
fn seat_position_maps_from_engine() {
    let pos = SeatPosition::from(EnginePosition::BigBlind);
    assert_eq!(serde_json::to_value(pos).unwrap(), "big_blind");

    let info = PlayerInfo {
        id: 0,
        stack: 20_000,
        position: pos,
        is_human: true,
    };
    let info_json = serde_json::to_value(info).unwrap();
    assert_eq!(info_json["position"], "big_blind");
}

fn card(suit: Suit, rank: Rank) -> Card {
    Card { suit, rank }
}
