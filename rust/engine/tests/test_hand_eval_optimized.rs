use axiomind_engine::cards::{Card, Rank as R, Suit as S};
use axiomind_engine::hand::{evaluate_hand, evaluate_hand_optimized};

fn c(s: S, r: R) -> Card {
    Card { suit: s, rank: r }
}

#[test]
fn optimized_matches_baseline_on_sample_hands() {
    let samples: Vec<[Card; 7]> = vec![
        [
            c(S::Hearts, R::Ten),
            c(S::Hearts, R::Jack),
            c(S::Hearts, R::Queen),
            c(S::Hearts, R::King),
            c(S::Hearts, R::Ace),
            c(S::Clubs, R::Two),
            c(S::Diamonds, R::Three),
        ],
        [
            c(S::Clubs, R::Ace),
            c(S::Diamonds, R::Ace),
            c(S::Hearts, R::Ace),
            c(S::Spades, R::Ace),
            c(S::Clubs, R::King),
            c(S::Diamonds, R::Queen),
            c(S::Hearts, R::Two),
        ],
        [
            c(S::Clubs, R::King),
            c(S::Diamonds, R::King),
            c(S::Hearts, R::King),
            c(S::Clubs, R::Queen),
            c(S::Diamonds, R::Queen),
            c(S::Hearts, R::Two),
            c(S::Spades, R::Three),
        ],
        [
            c(S::Hearts, R::Two),
            c(S::Hearts, R::Seven),
            c(S::Hearts, R::Jack),
            c(S::Hearts, R::Queen),
            c(S::Hearts, R::Nine),
            c(S::Clubs, R::Ace),
            c(S::Diamonds, R::King),
        ],
        [
            c(S::Clubs, R::Five),
            c(S::Hearts, R::Six),
            c(S::Clubs, R::Seven),
            c(S::Hearts, R::Eight),
            c(S::Diamonds, R::Nine),
            c(S::Spades, R::Two),
            c(S::Clubs, R::Three),
        ],
        [
            c(S::Clubs, R::Queen),
            c(S::Hearts, R::Queen),
            c(S::Diamonds, R::Queen),
            c(S::Spades, R::Two),
            c(S::Clubs, R::Three),
            c(S::Hearts, R::Four),
            c(S::Diamonds, R::Five),
        ],
        [
            c(S::Clubs, R::Ace),
            c(S::Hearts, R::Ace),
            c(S::Spades, R::Two),
            c(S::Diamonds, R::Three),
            c(S::Clubs, R::Four),
            c(S::Diamonds, R::Nine),
            c(S::Hearts, R::Seven),
        ],
        [
            c(S::Clubs, R::Ace),
            c(S::Hearts, R::King),
            c(S::Spades, R::Nine),
            c(S::Diamonds, R::Eight),
            c(S::Clubs, R::Seven),
            c(S::Diamonds, R::Three),
            c(S::Hearts, R::Two),
        ],
    ];
    for cards in samples.iter() {
        let base = evaluate_hand(cards);
        let opt = evaluate_hand_optimized(cards);
        assert_eq!(base.category, opt.category);
        assert_eq!(base.kickers, opt.kickers);
    }
}
