use axiomind_engine::pot::PotManager;

#[test]
fn heads_up_simple_side_pot() {
    let pm = PotManager::from_contributions([500, 1000]);
    assert_eq!(pm.main_pot(), 1000);
    assert_eq!(pm.side_pots(), &[500]);
}

#[test]
fn equal_stacks_no_side_pot() {
    let pm = PotManager::from_contributions([1000, 1000]);
    assert_eq!(pm.main_pot(), 2000);
    assert!(pm.side_pots().is_empty());
}
