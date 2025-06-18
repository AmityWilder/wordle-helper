use arrayvec::ArrayVec;
use bitflags::bitflags;
use crate::{dictionary::*, word::{Letter, Word}, VERBOSE_MESSAGES};

bitflags!{
  #[derive(Debug, Clone, Copy)]
  pub struct Positions: u8 {
    const P1 = 1 << 0;
    const P2 = 1 << 1;
    const P3 = 1 << 2;
    const P4 = 1 << 3;
    const P5 = 1 << 4;
  }
}

impl Positions {
  pub const fn from_index(index: usize) -> Option<Self> {
    Self::from_bits(1u8 << index)
  }

  pub const fn into_index(self) -> usize {
    debug_assert!(self.bits().count_ones() == 1);
    self.bits().trailing_zeros() as usize
  }
}

const _: () = {
  assert!(Positions::P1.into_index() == 0);
  assert!(Positions::P2.into_index() == 1);
  assert!(Positions::P3.into_index() == 2);
  assert!(Positions::P4.into_index() == 3);
  assert!(Positions::P5.into_index() == 4);
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharStatus {
  Excluded,
  Required,
  Confirmed,
}

impl std::fmt::Display for CharStatus {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CharStatus::Excluded => "⬜️".fmt(f),
      CharStatus::Required => "🟨".fmt(f),
      CharStatus::Confirmed => "🟩".fmt(f),
    }
  }
}

pub struct Guesser {
  candidates: Vec<Word>,
  /// Sorted alphabetically
  excluded: ArrayVec<Letter, {26 - 5}>,
  /// Sorted alphabetically
  required: ArrayVec<(Letter, Positions), 5>,
  confirmed: [Option<Letter>; 5],
}

impl Guesser {
  pub fn new(mut candidates_buf: Vec<Word>) -> Self {
    candidates_buf.clear();
    candidates_buf.extend_from_slice(FIVE_LETTER_WORDS.as_slice());
    Self {
      candidates: candidates_buf,
      excluded: ArrayVec::new(),
      required: ArrayVec::new(),
      confirmed: [const { None }; 5],
    }
  }

  #[cfg(test)]
  pub fn extract_resources(self) -> Vec<Word> {
    self.candidates
  }

  pub fn guess(&self, turn: u32) -> Option<&Word> {
    if turn <= 5 {
      self.candidates.first()
    } else {
      self.candidates.last()
    }
  }

  pub fn candidates(&self) -> &[Word] {
    &self.candidates
  }

  pub const fn confirmed_word(&self) -> Option<Word> {
    if let [Some(c1), Some(c2), Some(c3), Some(c4), Some(c5)] = self.confirmed {
      Some(Word([c1, c2, c3, c4, c5]))
    } else {
      None
    }
  }

  fn confirm(&mut self, idx: usize, ch: Letter) {
    self.confirmed[idx] = Some(ch);
    if VERBOSE_MESSAGES {
      println!("letter '{ch}' is confirmed at position {}", idx + 1);
    }
  }

  /// If only one possible space, treat as confirmed
  ///
  /// Returns `true` if an unknown was confirmed
  fn pidgeon(&mut self, idx: usize) -> bool {
    let (ch, p) = self.required[idx];
    let confirmed_positions = Positions::from_iter(
      self.confirmed.iter()
        .enumerate()
        .filter(|(_, c)| c.is_some_and(|c| c != ch))
        .map(|(i, _)| Positions::from_index(i).unwrap())
    );
    let possible_positions = p
      .union(confirmed_positions)
      .complement();
    let num_possible_positions = possible_positions.bits().count_ones();
    assert_ne!(num_possible_positions, 0, "letter '{ch}' has no possible placement");
    if VERBOSE_MESSAGES {
      println!("letter '{ch}' can only be placed in {possible_positions:?}");
    }
    if num_possible_positions == 1 {
      assert!(!possible_positions.is_empty());
      let only_open = possible_positions.into_index();
      if VERBOSE_MESSAGES {
        println!("letter '{ch}' can only be placed at position {}", only_open + 1);
      }
      self.confirm(only_open, ch);
      _ = self.required.remove(idx);
      true
    } else {
      false
    }
  }

  pub fn analyze(&mut self, chars: [(Letter, CharStatus); 5]) {
    for (i, (ch, stat)) in chars.into_iter().enumerate() {
      match stat {
        CharStatus::Excluded => {
          if let Err(pos) = self.excluded.binary_search(&ch) {
            self.excluded.insert(pos, ch);
            if VERBOSE_MESSAGES {
              println!("letter '{ch}' is not in the word");
            }
          }
        }

        CharStatus::Required => {
          let pos = Positions::from_index(i).unwrap();
          let idx = match self.required.binary_search_by_key(&ch, |(r, _)| *r) {
            Ok(idx) => { self.required[idx].1.insert(pos); idx },
            Err(idx) => { self.required.insert(idx, (ch, pos)); idx },
          };
          if VERBOSE_MESSAGES {
            println!("letter '{ch}' is required but cannot be in {:?}", self.required[idx].1);
          }
          _ = self.pidgeon(idx);
        }

        CharStatus::Confirmed => {
          self.confirm(i, ch);
          if let Ok(i) = self.required.binary_search_by_key(&ch, |(ch, _)| *ch) {
            if VERBOSE_MESSAGES {
              println!("letter '{ch}' no longer unknown");
            }
            _ = self.required.remove(i);
          }
        }
      }
    }

    if VERBOSE_MESSAGES {
      println!("draining...");
    }
    'outer: loop {
      for i in 0..self.required.len() {
        if self.pidgeon(i) {
          continue 'outer;
        }
      }
      break;
    }
    if VERBOSE_MESSAGES {
      println!("feedback complete");
    }
  }

  pub fn prune(&mut self) {
    let include = |word: &Word| -> bool {
      // Must contain all confirmed
      word.iter().copied().zip(self.confirmed.iter().copied())
        .all(|(a, b)| b.is_none_or(|b| a == b))
      &&
      // Must contain none excluded
      !word.iter().any(|ch| self.excluded.binary_search(ch).is_ok())
      &&
      // Must contain all required
      self.required.iter().copied().all(|(r, p)| {
        word.contains(&r) &&
        word.iter().copied()
          .enumerate()
          // but only in an open space
          .filter(|&(i, ch)| self.confirmed[i].is_none() && ch == r)
          // where that character has not been tried yet
          .all(|(i, _)| !p.contains(Positions::from_index(i).unwrap()))
      })
    };

    self.candidates.retain(include);
  }
}
