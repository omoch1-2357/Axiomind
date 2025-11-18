use axiomind_engine::engine::Engine;

#[test]
fn zero_stack_prevents_new_hand() {
    let mut eng = Engine::new(Some(1), 1);
    // Drain player 0 stack to zero
    {
        let players = eng.players_mut();
        let s0 = players[0].stack();
        players[0].bet(s0).expect("bet to zero");
    }
    // Next deal should refuse
    let r = eng.deal_hand();
    assert!(r.is_err(), "deal_hand should error when any stack is zero");
}
