use crate::{guess::{CharFeedback, WordFeedback}, word::Word};

pub struct Game {
  word: Word,
}

impl Game {
  pub const fn new(word: Word) -> Self {
    Self { word }
  }

  pub fn check(&self, guess: &Word) -> WordFeedback {
    WordFeedback(std::array::from_fn(|i| {
      let ch = guess[i];
      if self.word.contains(&ch) {
        if self.word[i] == ch {
          CharFeedback::Confirmed
        } else {
          CharFeedback::Required
        }
      } else {
        CharFeedback::Excluded
      }
    }))
  }
}
