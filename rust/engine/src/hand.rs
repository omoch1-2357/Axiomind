use std::cmp::Ordering;

use crate::cards::{Card, Rank, Suit};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Category {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct HandStrength {
    pub category: Category,
    // kickers: ordered high -> low for tiebreaks
    pub kickers: [u8; 5],
}

pub fn evaluate_hand(cards: &[Card; 7]) -> HandStrength {
    // Count ranks and suits
    let mut rank_counts = [0u8; 15]; // 2..14 used
    let mut suit_counts = [0u8; 4];
    let mut by_suit: [Vec<u8>; 4] = [vec![], vec![], vec![], vec![]];
    for &c in cards.iter() {
        let r = rank_val(c.rank);
        rank_counts[r as usize] += 1;
        let s = suit_index(c.suit);
        suit_counts[s] += 1;
        by_suit[s].push(r);
    }

    // Check flush and straight flush
    let mut flush_suit: Option<usize> = None;
    for (s, &count) in suit_counts.iter().enumerate() {
        if count >= 5 {
            flush_suit = Some(s);
            break;
        }
    }

    // Straight flush
    if let Some(s) = flush_suit {
        by_suit[s].sort_unstable();
        by_suit[s].dedup();
        if let Some(high) = detect_straight_high(&by_suit[s]) {
            return HandStrength {
                category: Category::StraightFlush,
                kickers: [high, 0, 0, 0, 0],
            };
        }
    }

    // Four of a kind
    if let Some((quad, kicker)) = detect_quads(&rank_counts) {
        return HandStrength {
            category: Category::FourOfAKind,
            kickers: [quad, kicker, 0, 0, 0],
        };
    }

    // Full house
    if let Some((trip, pair)) = detect_full_house(&rank_counts) {
        return HandStrength {
            category: Category::FullHouse,
            kickers: [trip, pair, 0, 0, 0],
        };
    }

    // Flush
    if let Some(s) = flush_suit {
        let mut ranks = by_suit[s].clone();
        ranks.sort_unstable_by(|a, b| b.cmp(a));
        let mut k = [0u8; 5];
        k.copy_from_slice(&ranks[..5]);
        return HandStrength {
            category: Category::Flush,
            kickers: k,
        };
    }

    // Straight
    let mut uniq: Vec<u8> = Vec::new();
    for r in (2..=14).rev() {
        if rank_counts[r as usize] > 0 {
            uniq.push(r as u8);
        }
    }
    uniq.sort_unstable();
    uniq.dedup();
    if let Some(high) = detect_straight_high(&uniq) {
        return HandStrength {
            category: Category::Straight,
            kickers: [high, 0, 0, 0, 0],
        };
    }

    // Three / Two pair / One pair / High card
    let (trip_ranks, pair_ranks, singles) = classify_multiples(&rank_counts);
    if let Some(t) = trip_ranks.first().copied() {
        // trips + two highest kickers
        let mut k = [t, 0, 0, 0, 0];
        let mut remain = vec![];
        remain.extend(pair_ranks.iter().copied());
        remain.extend(singles.iter().copied());
        remain.sort_unstable_by(|a, b| b.cmp(a));
        k[1] = *remain.first().unwrap_or(&0);
        k[2] = *remain.get(1).unwrap_or(&0);
        return HandStrength {
            category: Category::ThreeOfAKind,
            kickers: k,
        };
    }
    if pair_ranks.len() >= 2 {
        let mut prs = pair_ranks.clone();
        prs.sort_unstable();
        prs.reverse();
        let high = prs[0];
        let low = prs[1];
        let mut k = [high, low, 0, 0, 0];
        let mut rest = singles.clone();
        rest.sort_unstable_by(|a, b| b.cmp(a));
        k[2] = *rest.first().unwrap_or(&0);
        return HandStrength {
            category: Category::TwoPair,
            kickers: k,
        };
    }
    if let Some(p) = pair_ranks.first().copied() {
        let mut k = [p, 0, 0, 0, 0];
        let mut rest = singles.clone();
        rest.sort_unstable_by(|a, b| b.cmp(a));
        for i in 0..3 {
            k[i + 1] = *rest.get(i).unwrap_or(&0);
        }
        return HandStrength {
            category: Category::OnePair,
            kickers: k,
        };
    }

    // High card: top 5 ranks
    let mut highs = singles.clone();
    highs.sort_unstable_by(|a, b| b.cmp(a));
    let mut k = [0u8; 5];
    for (i, item) in k.iter_mut().enumerate() {
        *item = *highs.get(i).unwrap_or(&0);
    }
    HandStrength {
        category: Category::HighCard,
        kickers: k,
    }
}

pub fn compare_hands(a: &HandStrength, b: &HandStrength) -> Ordering {
    match a.category.cmp(&b.category) {
        Ordering::Equal => a.kickers.cmp(&b.kickers),
        ord => ord,
    }
}

// Optimized variant using bitmasks for straight detection paths.
pub fn evaluate_hand_optimized(cards: &[Card; 7]) -> HandStrength {
    // leverage the same logic but use bit masks to speed straight/flush checks
    // Build rank bitset and suit counts
    let mut _rank_mask: u16 = 0;
    let mut suit_counts = [0u8; 4];
    let mut by_suit_mask: [u16; 4] = [0, 0, 0, 0];
    for &c in cards.iter() {
        let r = rank_val(c.rank);
        _rank_mask |= 1u16 << r;
        let s = suit_index(c.suit);
        suit_counts[s] += 1;
        by_suit_mask[s] |= 1u16 << r;
    }

    // Straight flush via mask
    for s in 0..4 {
        if suit_counts[s] >= 5 {
            if let Some(high) = straight_high_from_mask(by_suit_mask[s]) {
                return HandStrength {
                    category: Category::StraightFlush,
                    kickers: [high, 0, 0, 0, 0],
                };
            }
        }
    }

    // Fallback to baseline for the rest (keeps equivalence simple)
    evaluate_hand(cards)
}

fn rank_val(r: Rank) -> u8 {
    r as u8
}
fn suit_index(s: Suit) -> usize {
    match s {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

fn detect_straight_high(sorted_unique_ranks: &[u8]) -> Option<u8> {
    if sorted_unique_ranks.is_empty() {
        return None;
    }
    // Ensure ascending order
    let mut v = sorted_unique_ranks.to_vec();
    v.sort_unstable();
    // Ace-low straight support: treat Ace as 1 additionally
    let mut w = v.clone();
    if v.binary_search(&14).is_ok() {
        w.insert(0, 1);
    }

    let mut run = 1;
    let mut best_high = 0u8;
    for i in 1..w.len() {
        if w[i] == w[i - 1] + 1 {
            run += 1;
            if run >= 5 {
                best_high = w[i];
            }
        } else if w[i] != w[i - 1] {
            // break in sequence
            run = 1;
        }
    }
    if best_high == 0 {
        None
    } else {
        Some(if best_high == 5 { 5 } else { best_high })
    }
}

fn straight_high_from_mask(mask: u16) -> Option<u8> {
    // Treat Ace as 14 and optionally as 1
    let mut m = mask;
    // add Ace-low if Ace present
    if (m & (1 << 14)) != 0 {
        m |= 1 << 1;
    }
    // Sliding 5-bit window from Ace(14) down to 5
    for high in (5..=14).rev() {
        let window = (1u16 << (high - 4))
            | (1 << (high - 3))
            | (1 << (high - 2))
            | (1 << (high - 1))
            | (1 << high);
        if (m & window) == window {
            return Some(if high == 5 { 5 } else { high as u8 });
        }
    }
    None
}

pub fn evaluate_many_optimized(cards: &[Card; 7], n: usize) -> Vec<HandStrength> {
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push(evaluate_hand_optimized(cards));
    }
    v
}

fn detect_quads(rank_counts: &[u8; 15]) -> Option<(u8, u8)> {
    let mut quad = 0u8;
    let mut kicker = 0u8;
    for r in (2..=14).rev() {
        let c = rank_counts[r as usize];
        if c == 4 {
            quad = r as u8;
            break;
        }
    }
    if quad == 0 {
        return None;
    }
    for r in (2..=14).rev() {
        if r as u8 != quad && rank_counts[r as usize] > 0 {
            kicker = r as u8;
            break;
        }
    }
    Some((quad, kicker))
}

fn detect_full_house(rank_counts: &[u8; 15]) -> Option<(u8, u8)> {
    let mut trips: Vec<u8> = vec![];
    let mut pairs: Vec<u8> = vec![];
    for r in (2..=14).rev() {
        match rank_counts[r as usize] {
            3 => trips.push(r as u8),
            2 => pairs.push(r as u8),
            _ => {}
        }
    }
    if trips.is_empty() {
        return None;
    }
    if trips.len() >= 2 {
        let t = trips[0];
        let p = trips[1];
        return Some((t, p));
    }
    if !pairs.is_empty() {
        return Some((trips[0], pairs[0]));
    }
    None
}

fn classify_multiples(rank_counts: &[u8; 15]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut trips = vec![];
    let mut pairs = vec![];
    let mut singles = vec![];
    for r in 2..=14 {
        match rank_counts[r as usize] {
            3 => trips.push(r as u8),
            2 => pairs.push(r as u8),
            1 => singles.push(r as u8),
            _ => {}
        }
    }
    (trips, pairs, singles)
}
