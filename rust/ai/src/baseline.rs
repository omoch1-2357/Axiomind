//! Baseline AI implementation for poker gameplay.
//!
//! Provides a simple AI opponent that can be used for testing and benchmarking.
//! Implements a basic rule-based strategy with hand evaluation and pot odds calculation.

use crate::AIOpponent;
use axiomind_engine::cards::{Card, Rank};
use axiomind_engine::engine::Engine;
use axiomind_engine::hand::{evaluate_hand, Category};
use axiomind_engine::logger::Street;
use axiomind_engine::player::PlayerAction;

/// Simple baseline AI implementation for testing and comparison.
///
/// This AI serves as a reference implementation and baseline for performance
/// comparison. It implements a basic rule-based strategy with:
/// - Preflop hand strength evaluation
/// - Postflop hand evaluation using board cards
/// - Pot odds calculation for calling decisions
/// - Deterministic decision making for reproducible simulations
///
/// # Strategy
///
/// **Preflop:**
/// - Strong hands (high pairs 77+, AK, AQ): Raise or call
/// - Medium hands (suited connectors, Ax, small pairs): Call if cheap
/// - Weak hands: Fold to raises, check if free
///
/// **Postflop:**
/// - Strong hands (Two Pair+): Bet or call
/// - Medium hands (One Pair): Check or call small bets
/// - Draws and weak hands: Calculate pot odds, fold if unfavorable
///
/// # Example
///
/// ```rust
/// use axiomind_ai::baseline::BaselineAI;
/// use axiomind_ai::AIOpponent;
/// use axiomind_engine::engine::Engine;
///
/// let ai = BaselineAI::new();
/// assert_eq!(ai.name(), "BaselineAI");
///
/// let mut engine = Engine::new(Some(42), 1);
/// engine.shuffle();
/// engine.deal_hand().expect("Failed to deal hand");
///
/// let player_id = engine.current_player().expect("No current player");
/// let action = ai.get_action(&engine, player_id);
/// // Action will be determined by hand strength and game state
/// ```
#[derive(Debug, Clone)]
pub struct BaselineAI;

impl BaselineAI {
    /// Create a new BaselineAI instance.
    ///
    /// # Returns
    ///
    /// A new `BaselineAI` ready to make decisions
    ///
    /// # Example
    ///
    /// ```rust
    /// use axiomind_ai::baseline::BaselineAI;
    ///
    /// let ai = BaselineAI::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Evaluate preflop hand strength on a scale of 0-10.
    ///
    /// # Arguments
    ///
    /// * `hole_cards` - Array of two hole cards
    ///
    /// # Returns
    ///
    /// Hand strength rating:
    /// - 9-10: Premium hands (AA, KK, QQ, JJ, AKs)
    /// - 7-8: Strong hands (TT-99, AK, AQ, KQ)
    /// - 5-6: Medium hands (88-77, AJ, suited connectors)
    /// - 3-4: Marginal hands (66-22, Ax, suited cards)
    /// - 0-2: Weak hands (offsuit low cards)
    fn evaluate_preflop_strength(hole_cards: [Card; 2]) -> u8 {
        let c1 = hole_cards[0];
        let c2 = hole_cards[1];

        let r1 = c1.rank as u8;
        let r2 = c2.rank as u8;
        let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
        let suited = c1.suit == c2.suit;

        // Pairs
        if r1 == r2 {
            return match high {
                14 => 10, // AA
                13 => 10, // KK
                12 => 9,  // QQ
                11 => 9,  // JJ
                10 => 8,  // TT
                9 => 7,   // 99
                8 => 6,   // 88
                7 => 5,   // 77
                _ => 4,   // 66-22
            };
        }

        // High card combinations
        match (high, low) {
            // AK, AQ, AJ, AT
            (14, 13) => {
                if suited {
                    10
                } else {
                    8
                }
            }
            (14, 12) => {
                if suited {
                    8
                } else {
                    7
                }
            }
            (14, 11) => {
                if suited {
                    7
                } else {
                    6
                }
            }
            (14, 10) => {
                if suited {
                    6
                } else {
                    5
                }
            }
            (14, _) => {
                if suited {
                    5
                } else {
                    4
                }
            } // Ax

            // KQ, KJ, KT
            (13, 12) => {
                if suited {
                    7
                } else {
                    6
                }
            }
            (13, 11) => {
                if suited {
                    6
                } else {
                    5
                }
            }
            (13, 10) => {
                if suited {
                    5
                } else {
                    4
                }
            }

            // QJ, QT
            (12, 11) => {
                if suited {
                    6
                } else {
                    5
                }
            }
            (12, 10) => {
                if suited {
                    5
                } else {
                    4
                }
            }

            // Suited connectors and one-gappers
            _ => {
                if suited && high - low <= 2 {
                    if high >= 9 {
                        5
                    } else {
                        4
                    }
                } else if high >= 11 && low >= 9 {
                    4 // Broadway cards
                } else {
                    2 // Weak offsuit
                }
            }
        }
    }

    /// Evaluate postflop hand strength using 7-card evaluation.
    ///
    /// # Arguments
    ///
    /// * `hole_cards` - Two hole cards
    /// * `board` - Community cards on the board
    ///
    /// # Returns
    ///
    /// Hand strength on scale of 0-10, or None if board has fewer than 3 cards
    fn evaluate_postflop_strength(hole_cards: [Card; 2], board: &[Card]) -> Option<u8> {
        if board.len() < 3 {
            return None;
        }

        // For turn and river with <5 board cards, use what we have + dummy cards
        let mut seven_cards = vec![hole_cards[0], hole_cards[1]];
        seven_cards.extend_from_slice(board);

        // Pad with dummy cards if needed (won't affect relative hand strength much)
        while seven_cards.len() < 7 {
            // Add dummy low cards that are unlikely to affect evaluation
            seven_cards.push(Card {
                suit: axiomind_engine::cards::Suit::Clubs,
                rank: Rank::Two,
            });
        }

        let cards_array: [Card; 7] = seven_cards[..7].try_into().ok()?;
        let strength = evaluate_hand(&cards_array);

        // Convert category to 0-10 scale
        let base_strength = match strength.category {
            Category::HighCard => 1,
            Category::OnePair => 3,
            Category::TwoPair => 5,
            Category::ThreeOfAKind => 6,
            Category::Straight => 7,
            Category::Flush => 8,
            Category::FullHouse => 9,
            Category::FourOfAKind => 10,
            Category::StraightFlush => 10,
        };

        // Adjust for kicker strength within same category
        let kicker_boost = if strength.kickers[0] >= 12 { 1 } else { 0 };

        Some((base_strength + kicker_boost).min(10))
    }

    /// Calculate pot odds for calling a bet.
    ///
    /// # Arguments
    ///
    /// * `pot_size` - Current pot size
    /// * `call_amount` - Amount required to call
    ///
    /// # Returns
    ///
    /// Pot odds as a ratio (pot / (pot + call)), used to determine if call is +EV
    fn calculate_pot_odds(pot_size: u32, call_amount: u32) -> f32 {
        if call_amount == 0 {
            return 1.0;
        }
        pot_size as f32 / (pot_size + call_amount) as f32
    }

    /// Make a decision based on game state and hand strength.
    ///
    /// # Arguments
    ///
    /// * `hand_strength` - Evaluated hand strength (0-10)
    /// * `to_call` - Amount needed to call
    /// * `min_raise` - Minimum raise amount
    /// * `stack` - Player's remaining stack
    /// * `pot` - Current pot size
    ///
    /// # Returns
    ///
    /// The chosen `PlayerAction`
    fn decide_action(
        hand_strength: u8,
        to_call: u32,
        min_raise: u32,
        stack: u32,
        pot: u32,
    ) -> PlayerAction {
        // Check if we can check for free
        if to_call == 0 {
            return Self::decide_no_bet_action(hand_strength, min_raise, stack, pot);
        }

        // Facing a bet - calculate pot odds
        let pot_odds = Self::calculate_pot_odds(pot, to_call);

        // If we don't have enough chips to call, must fold or go all-in
        if to_call > stack {
            return if hand_strength >= 7 {
                PlayerAction::AllIn
            } else {
                PlayerAction::Fold
            };
        }

        // Decision based on hand strength
        match hand_strength {
            // Very strong hands (9-10): Raise or call
            9..=10 => {
                if stack >= to_call + min_raise {
                    let raise_amount = (pot / 2).max(min_raise).min(stack - to_call);
                    if raise_amount >= min_raise {
                        return PlayerAction::Raise(raise_amount);
                    }
                }
                PlayerAction::Call
            }
            // Strong hands (7-8): Always call (deterministic)
            7..=8 => PlayerAction::Call,
            // Medium hands (5-6): Call if pot odds favorable
            5..=6 => {
                if pot_odds >= 0.3 || to_call <= pot / 4 {
                    PlayerAction::Call
                } else {
                    PlayerAction::Fold
                }
            }
            // Marginal hands (3-4): Call only if very cheap
            3..=4 => {
                if pot_odds >= 0.4 || to_call <= pot / 6 {
                    PlayerAction::Call
                } else {
                    PlayerAction::Fold
                }
            }
            // Weak hands (0-2): Always fold (no random bluffs)
            _ => PlayerAction::Fold,
        }
    }

    /// Decide action when there's no bet to call (can check for free).
    fn decide_no_bet_action(
        hand_strength: u8,
        min_raise: u32,
        stack: u32,
        pot: u32,
    ) -> PlayerAction {
        match hand_strength {
            // Very strong hands: Bet for value
            9..=10 => {
                if stack >= min_raise {
                    let bet_size = (pot * 2 / 3).max(min_raise).min(stack);
                    PlayerAction::Bet(bet_size)
                } else {
                    PlayerAction::Check
                }
            }
            // Strong hands: Always bet (deterministic)
            7..=8 => {
                if stack >= min_raise {
                    let bet_size = (pot / 2).max(min_raise).min(stack);
                    PlayerAction::Bet(bet_size)
                } else {
                    PlayerAction::Check
                }
            }
            // Medium hands: Always check (deterministic)
            5..=6 => PlayerAction::Check,
            // Weak/marginal hands: Always check (no random bluffs)
            _ => PlayerAction::Check,
        }
    }
}

impl Default for BaselineAI {
    fn default() -> Self {
        Self::new()
    }
}

impl AIOpponent for BaselineAI {
    /// Get the next action for the baseline AI.
    ///
    /// Implements a complete decision-making process:
    /// 1. Determine current street (preflop vs postflop)
    /// 2. Evaluate hand strength appropriately
    /// 3. Check game state (pot, to_call, stack)
    /// 4. Make deterministic decision based on hand strength and pot odds
    ///
    /// # Arguments
    ///
    /// * `engine` - Reference to the game engine
    /// * `player_id` - The player ID making the decision
    ///
    /// # Returns
    ///
    /// A valid `PlayerAction` that will not cause the game to crash
    fn get_action(&self, engine: &Engine, player_id: usize) -> PlayerAction {
        // Get player info
        let players = engine.players();
        let player = &players[player_id];

        // Extract hole cards - if not available, default to conservative play
        let hole_cards_opt = player.hole_cards();
        let hole_cards = match (hole_cards_opt[0], hole_cards_opt[1]) {
            (Some(c1), Some(c2)) => [c1, c2],
            _ => {
                // No hole cards, default to check/fold
                return if engine.to_call(player_id).unwrap_or(0) == 0 {
                    PlayerAction::Check
                } else {
                    PlayerAction::Fold
                };
            }
        };

        // Get game state
        let to_call = engine.to_call(player_id).unwrap_or(0);
        let min_raise = engine.min_raise().unwrap_or(100);
        let stack = player.stack();
        let pot = engine.pot();
        let board = engine.board();
        let street = engine.current_street();

        // Evaluate hand strength based on street
        let hand_strength = if street.is_none() || street == Some(Street::Preflop) {
            // Preflop evaluation
            Self::evaluate_preflop_strength(hole_cards)
        } else {
            // Postflop evaluation
            Self::evaluate_postflop_strength(hole_cards, board).unwrap_or_else(|| {
                // Fallback to preflop if board evaluation fails
                Self::evaluate_preflop_strength(hole_cards)
            })
        };

        // Make deterministic decision based on all factors
        Self::decide_action(hand_strength, to_call, min_raise, stack, pot)
    }

    /// Return the name of this AI implementation.
    ///
    /// # Returns
    ///
    /// The string "BaselineAI"
    fn name(&self) -> &str {
        "BaselineAI"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiomind_engine::cards::{Rank, Suit};

    #[test]
    fn test_baseline_ai_creation() {
        let ai = BaselineAI::new();
        assert_eq!(ai.name(), "BaselineAI");
    }

    #[test]
    fn test_baseline_ai_default() {
        let ai = BaselineAI;
        assert_eq!(ai.name(), "BaselineAI");
    }

    #[test]
    fn test_preflop_strength_premium_pairs() {
        let aces = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ];
        assert_eq!(BaselineAI::evaluate_preflop_strength(aces), 10);

        let kings = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::King,
            },
        ];
        assert_eq!(BaselineAI::evaluate_preflop_strength(kings), 10);
    }

    #[test]
    fn test_preflop_strength_ace_king() {
        let ak_suited = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
        ];
        assert_eq!(BaselineAI::evaluate_preflop_strength(ak_suited), 10);

        let ak_offsuit = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::King,
            },
        ];
        assert_eq!(BaselineAI::evaluate_preflop_strength(ak_offsuit), 8);
    }

    #[test]
    fn test_preflop_strength_weak_hands() {
        let weak = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Seven,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Two,
            },
        ];
        assert!(BaselineAI::evaluate_preflop_strength(weak) <= 3);
    }

    #[test]
    fn test_pot_odds_calculation() {
        // Pot is 100, call is 50 -> pot odds = 100/150 = 0.667
        let odds = BaselineAI::calculate_pot_odds(100, 50);
        assert!((odds - 0.667).abs() < 0.01);

        // Pot is 200, call is 100 -> pot odds = 200/300 = 0.667
        let odds2 = BaselineAI::calculate_pot_odds(200, 100);
        assert!((odds2 - 0.667).abs() < 0.01);

        // Free action (call = 0)
        let odds3 = BaselineAI::calculate_pot_odds(100, 0);
        assert_eq!(odds3, 1.0);
    }

    #[test]
    fn test_postflop_strength_full_board() {
        let hole = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ];
        let board = vec![
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ten,
            },
        ];

        let strength = BaselineAI::evaluate_postflop_strength(hole, &board);
        assert!(strength.is_some());
        assert!(strength.unwrap() >= 6); // Three of a kind or better
    }

    #[test]
    fn test_baseline_ai_action_with_hole_cards() {
        let ai = BaselineAI::new();
        let mut engine = Engine::new(Some(42), 1);

        engine.shuffle();
        engine.deal_hand().expect("Failed to deal hand");

        let player_id = engine.current_player().expect("No current player");
        let action = ai.get_action(&engine, player_id);

        // Should return a valid action (not panic)
        // Action should be one of the valid types
        match action {
            PlayerAction::Fold
            | PlayerAction::Check
            | PlayerAction::Call
            | PlayerAction::Bet(_)
            | PlayerAction::Raise(_)
            | PlayerAction::AllIn => {
                // Valid action
            }
        }
    }

    #[test]
    fn test_baseline_ai_handles_no_hole_cards() {
        let ai = BaselineAI::new();
        let engine = Engine::new(Some(42), 1);

        // No hand dealt yet
        let action = ai.get_action(&engine, 0);

        // Should default to check or fold without panicking
        match action {
            PlayerAction::Check | PlayerAction::Fold => {
                // Expected behavior
            }
            _ => panic!("Unexpected action when no hole cards present"),
        }
    }

    #[test]
    fn test_suited_connectors() {
        let suited_conn = [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Eight,
            },
        ];
        let strength = BaselineAI::evaluate_preflop_strength(suited_conn);
        assert!((4..=6).contains(&strength));
    }
}
