use std::collections::HashSet;

// Failing tests first for Task 2.1 (card & deck)

use axiomind_engine::cards::Card;
use axiomind_engine::deck::Deck;

#[test]
fn deck_reset_has_52_unique_cards() {
    let mut deck = Deck::new_with_seed(42);
    deck.reset();
    let mut set = HashSet::new();
    for i in 0..52 {
        let c = deck.deal_card().expect("should have 52 cards");
        assert!(set.insert(c), "card {:?} duplicated at position {}", c, i);
    }
    assert!(
        deck.deal_card().is_none(),
        "after 52 cards, deck should be empty"
    );
}

#[test]
fn shuffle_is_deterministic_with_same_seed() {
    let mut d1 = Deck::new_with_seed(12345);
    let mut d2 = Deck::new_with_seed(12345);
    d1.shuffle();
    d2.shuffle();
    // Compare first 10 cards
    let a: Vec<Card> = (0..10).map(|_| d1.deal_card().unwrap()).collect();
    let b: Vec<Card> = (0..10).map(|_| d2.deal_card().unwrap()).collect();
    assert_eq!(a, b, "same seed must yield identical order");
}

#[test]
fn shuffle_differs_with_different_seed() {
    let mut d1 = Deck::new_with_seed(1);
    let mut d2 = Deck::new_with_seed(2);
    d1.shuffle();
    d2.shuffle();
    let a: Vec<Card> = (0..10).map(|_| d1.deal_card().unwrap()).collect();
    let b: Vec<Card> = (0..10).map(|_| d2.deal_card().unwrap()).collect();
    assert_ne!(
        a, b,
        "different seeds should produce different orders (high probability)"
    );
}

#[test]
fn burn_and_deal_follow_holdem_procedure() {
    let mut deck = Deck::new_with_seed(777);
    deck.shuffle();

    // preflop: deal 2 each
    let p1 = [deck.deal_card().unwrap(), deck.deal_card().unwrap()];
    let p2 = [deck.deal_card().unwrap(), deck.deal_card().unwrap()];
    assert_ne!(p1, p2);

    // flop
    deck.burn_card();
    let flop = [
        deck.deal_card().unwrap(),
        deck.deal_card().unwrap(),
        deck.deal_card().unwrap(),
    ];
    // turn
    deck.burn_card();
    let turn = deck.deal_card().unwrap();
    // river
    deck.burn_card();
    let river = deck.deal_card().unwrap();

    // Ensure all these cards are unique
    let mut set = HashSet::new();
    for c in [
        p1[0], p1[1], p2[0], p2[1], flop[0], flop[1], flop[2], turn, river,
    ] {
        assert!(set.insert(c));
    }
}
