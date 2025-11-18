use axiomind_engine::engine::Engine;

#[test]
fn new_engine_initializes_two_players_with_20000() {
    let eng = Engine::new(Some(1234), 1);
    let stacks: Vec<u32> = eng.players().iter().map(|p| p.stack()).collect();
    assert_eq!(stacks, vec![20_000, 20_000]);
}

#[test]
fn same_seed_produces_deterministic_deal_order() {
    let mut e1 = Engine::new(Some(42), 1);
    let mut e2 = Engine::new(Some(42), 1);
    e1.shuffle();
    e2.shuffle();
    let a = e1.draw_n(5);
    let b = e2.draw_n(5);
    assert_eq!(a, b);
}
