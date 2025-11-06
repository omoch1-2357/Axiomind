use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::cards::{full_deck, Card};

#[derive(Debug)]
pub struct Deck {
    cards: Vec<Card>,
    position: usize,
    rng: ChaCha20Rng,
}

impl Deck {
    pub fn new_with_seed(seed: u64) -> Self {
        let rng = ChaCha20Rng::seed_from_u64(seed);
        // Keep initial order until shuffle is called explicitly
        Self {
            cards: full_deck(),
            position: 0,
            rng,
        }
    }

    pub fn shuffle(&mut self) {
        self.cards = full_deck();
        self.cards.shuffle(&mut self.rng);
        self.position = 0;
    }

    pub fn deal_card(&mut self) -> Option<Card> {
        if self.position >= self.cards.len() {
            None
        } else {
            let c = self.cards[self.position];
            self.position += 1;
            Some(c)
        }
    }

    pub fn burn_card(&mut self) {
        let _ = self.deal_card();
    }

    pub fn reset(&mut self) {
        self.cards = full_deck();
        self.position = 0;
    }

    #[allow(dead_code)]
    pub fn remaining(&self) -> usize {
        self.cards.len().saturating_sub(self.position)
    }
}
