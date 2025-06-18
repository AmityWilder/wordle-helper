use crate::guess::CharStatus;

pub struct Player {
  word: [u8; 5],
}

impl Player {
  pub const fn new(word: [u8; 5]) -> Self {
    Self {
      word,
    }
  }

  pub fn check(&self, guess: &[u8; 5]) -> [CharStatus; 5] {
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
