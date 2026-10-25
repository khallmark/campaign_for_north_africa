//! Recorded ChaCha8 stream-0 words; never uses an OS seed or platform-sized sampling.
use super::{reducer::Unavailable, state::RngState};
use crate::contract::command::RandomDraw;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha8Rng,
};
use std::num::NonZeroU32;

pub struct RecordedRng {
    generator: ChaCha8Rng,
    state: RngState,
    draws: Vec<RandomDraw>,
    pub(crate) failed: bool,
}

impl RecordedRng {
    pub fn new(state: &RngState) -> Self {
        let mut generator = ChaCha8Rng::from_seed(state.seed);
        generator.set_stream(0);
        generator.set_word_pos(u128::from(state.word_position));
        Self {
            generator,
            state: state.clone(),
            draws: Vec::new(),
            failed: false,
        }
    }

    pub fn next_word(&mut self) -> Result<u32, Unavailable> {
        let Some(next) = self.state.word_position.checked_add(1) else {
            self.failed = true;
            return Err(Unavailable);
        };
        let value = self.generator.next_u32();
        self.draws.push(RandomDraw {
            word_index: self.state.word_position,
            value,
        });
        self.state.word_position = next;
        Ok(value)
    }

    /// Uniform integer in 0..upper. Discard the short prefix of the u32 range
    /// so the remaining count is divisible by upper. ALL words stay recorded.
    /// Even upper=1 consumes a word; game-specific dice add their offset later.
    pub fn bounded(&mut self, upper: NonZeroU32) -> Result<u32, Unavailable> {
        let upper = upper.get();
        let threshold = upper.wrapping_neg() % upper;
        loop {
            let word = self.next_word()?;
            if word >= threshold {
                return Ok(word % upper);
            }
        }
    }

    pub fn state(&self) -> &RngState {
        &self.state
    }
    pub fn draws(&self) -> &[RandomDraw] {
        &self.draws
    }

    pub(crate) fn into_parts(self) -> (RngState, Vec<RandomDraw>) {
        (self.state, self.draws)
    }
}
