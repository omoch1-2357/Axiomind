use axiomind_engine::engine::Engine;

#[test]
fn burn_cards_and_board_count_are_correct() {
    let mut eng = Engine::new(Some(123), 1);
    eng.shuffle();
    eng.deal_hand().expect("deal_hand should succeed");
    // 5 board cards should be present
    assert_eq!(eng.board().len(), 5);
    // each player has 2 hole cards
    let players = eng.players();
    assert!(players[0].hole_cards()[0].is_some() && players[0].hole_cards()[1].is_some());
    assert!(players[1].hole_cards()[0].is_some() && players[1].hole_cards()[1].is_some());
    // Remaining cards: 52 - 4 (holes) - 3 (flop) - 1 (turn) - 1 (river) - 3 burns = 40
    assert_eq!(eng.deck_remaining(), 40);
}
