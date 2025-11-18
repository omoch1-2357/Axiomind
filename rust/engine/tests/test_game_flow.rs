use axiomind_engine::engine::Engine;

#[test]
fn deal_hand_progresses_streets_and_completes() {
    let mut eng = Engine::new(Some(1), 1);
    eng.shuffle();
    eng.deal_hand().expect("deal ok");
    let board = eng.board();
    assert_eq!(board.len(), 5);
    let players = eng.players();
    assert!(players
        .iter()
        .all(|p| p.hole_cards()[0].is_some() && p.hole_cards()[1].is_some()));
    assert!(eng.is_hand_complete());
}
