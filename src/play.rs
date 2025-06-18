use crate::{guess::CharStatus, word::Word};

pub struct Player {
  word: Word,
}

impl Player {
  pub const fn new(word: Word) -> Self {
    Self {
      word,
    }
  }

  pub fn check(&self, guess: &Word) -> [CharStatus; 5] {
    std::array::from_fn(|i| {
      let ch = guess[i];
      if self.word.contains(&ch) {
        if self.word[i] == ch {
          CharStatus::Confirmed
        } else {
          CharStatus::Required
        }
      } else {
        CharStatus::Excluded
      }
    })
  }
}
