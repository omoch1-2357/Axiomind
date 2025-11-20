use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::hand::{HandStrength, evaluate_many_optimized};

fn c(s: S, r: R) -> Card {
    Card { suit: s, rank: r }
}

#[test]
fn batch_optimized_evaluates_n_hands() {
    let cards = [
        c(S::Hearts, R::Ten),
        c(S::Hearts, R::Jack),
        c(S::Hearts, R::Queen),
        c(S::Hearts, R::King),
        c(S::Hearts, R::Ace),
        c(S::Clubs, R::Two),
        c(S::Diamonds, R::Three),
    ];
    let out: Vec<HandStrength> = evaluate_many_optimized(&cards, 5);
    assert_eq!(out.len(), 5);
}
