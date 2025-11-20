use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::hand::{Category, compare_hands, evaluate_hand};

fn c(s: S, r: R) -> Card {
    Card { suit: s, rank: r }
}

#[test]
fn detects_royal_flush() {
    let cards = [
        c(S::Hearts, R::Ten),
        c(S::Hearts, R::Jack),
        c(S::Hearts, R::Queen),
        c(S::Hearts, R::King),
        c(S::Hearts, R::Ace),
        c(S::Clubs, R::Two),
        c(S::Diamonds, R::Three),
    ];
    let hs = evaluate_hand(&cards);
    assert_eq!(hs.category, Category::StraightFlush);
}

#[test]
fn category_ordering_is_correct() {
    // Four of a kind vs full house
    let quads = [
        c(S::Clubs, R::Ace),
        c(S::Diamonds, R::Ace),
        c(S::Hearts, R::Ace),
        c(S::Spades, R::Ace),
        c(S::Clubs, R::King),
        c(S::Diamonds, R::Queen),
        c(S::Hearts, R::Two),
    ];
    let full_house = [
        c(S::Clubs, R::King),
        c(S::Diamonds, R::King),
        c(S::Hearts, R::King),
        c(S::Clubs, R::Queen),
        c(S::Diamonds, R::Queen),
        c(S::Hearts, R::Two),
        c(S::Spades, R::Three),
    ];
    let a = evaluate_hand(&quads);
    let b = evaluate_hand(&full_house);
    assert!(compare_hands(&a, &b).is_gt());
}

#[test]
fn straight_beats_three_of_a_kind() {
    let straight = [
        c(S::Clubs, R::Five),
        c(S::Hearts, R::Six),
        c(S::Clubs, R::Seven),
        c(S::Hearts, R::Eight),
        c(S::Diamonds, R::Nine),
        c(S::Spades, R::Two),
        c(S::Clubs, R::Three),
    ];
    let trips = [
        c(S::Clubs, R::Queen),
        c(S::Hearts, R::Queen),
        c(S::Diamonds, R::Queen),
        c(S::Spades, R::Two),
        c(S::Clubs, R::Three),
        c(S::Hearts, R::Four),
        c(S::Diamonds, R::Five),
    ];
    let a = evaluate_hand(&straight);
    let b = evaluate_hand(&trips);
    assert!(compare_hands(&a, &b).is_gt());
}

#[test]
fn flush_beats_straight_and_is_detected() {
    let flush = [
        c(S::Hearts, R::Two),
        c(S::Hearts, R::Seven),
        c(S::Hearts, R::Jack),
        c(S::Hearts, R::Queen),
        c(S::Hearts, R::Nine),
        c(S::Clubs, R::Ace),
        c(S::Diamonds, R::King),
    ];
    let straight = [
        c(S::Clubs, R::Five),
        c(S::Hearts, R::Six),
        c(S::Clubs, R::Seven),
        c(S::Hearts, R::Eight),
        c(S::Diamonds, R::Nine),
        c(S::Spades, R::Two),
        c(S::Clubs, R::Three),
    ];
    let a = evaluate_hand(&flush);
    assert_eq!(a.category, Category::Flush);
    let b = evaluate_hand(&straight);
    assert!(compare_hands(&a, &b).is_gt());
}

#[test]
fn pair_vs_high_card() {
    let pair = [
        c(S::Clubs, R::Ace),
        c(S::Hearts, R::Ace),
        c(S::Spades, R::Two),
        c(S::Diamonds, R::Three),
        c(S::Clubs, R::Four),
        c(S::Diamonds, R::Five),
        c(S::Hearts, R::Seven),
    ];
    let high = [
        c(S::Clubs, R::Ace),
        c(S::Hearts, R::King),
        c(S::Spades, R::Nine),
        c(S::Diamonds, R::Eight),
        c(S::Clubs, R::Seven),
        c(S::Diamonds, R::Three),
        c(S::Hearts, R::Two),
    ];
    let a = evaluate_hand(&pair);
    let b = evaluate_hand(&high);
    assert!(compare_hands(&a, &b).is_gt());
}
