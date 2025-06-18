use arrayvec::ArrayVec;
use bitflags::bitflags;
use crate::{dictionary::*, VERBOSE_MESSAGES};

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

pub fn no_repeated_letters(word: &[u8; 5]) -> bool {
  for i in 1..5 {
    if word[..i].contains(&word[i]) {
      return false;
    }
  }
  true
}

pub struct Guesser {
  candidates: Vec<[u8; 5]>,
  /// Sorted alphabetically
  excluded: ArrayVec<u8, {26 - 5}>,
  /// Sorted alphabetically
  required: ArrayVec<(u8, Positions), 5>,
  confirmed: [Option<u8>; 5],
}

fn score(word: &[u8; 5]) -> f32 {
  let mut score = word.iter()
    .enumerate()
    .map(|item| match item {
      (0, b'A') => 5.7,
      (_, b'A') => 7.8,
      (0, b'B') => 6.0,
      (_, b'B') => 2.0,
      (0, b'C') => 9.4,
      (_, b'C') => 4.0,
      (0, b'D') => 6.1,
      (_, b'D') => 3.8,
      (0, b'E') => 3.9,
      (_, b'E') => 11.0,
      (0, b'F') => 4.1,
      (_, b'F') => 1.4,
      (0, b'G') => 3.3,
      (_, b'G') => 3.0,
      (0, b'H') => 3.7,
      (_, b'H') => 2.3,
      (0, b'I') => 3.9,
      (_, b'I') => 8.6,
      (0, b'J') => 1.1,
      (_, b'J') => 0.21,
      (0, b'K') => 1.0,
      (_, b'K') => 0.97,
      (0, b'L') => 3.1,
      (_, b'L') => 5.3,
      (0, b'M') => 5.6,
      (_, b'M') => 2.7,
      (0, b'N') => 2.2,
      (_, b'N') => 7.2,
      (0, b'O') => 2.5,
      (_, b'O') => 6.1,
      (0, b'P') => 7.7,
      (_, b'P') => 2.8,
      (0, b'Q') => 0.49,
      (_, b'Q') => 0.19,
      (0, b'R') => 6.0,
      (_, b'R') => 7.3,
      (0, b'S') => 11.0,
      (_, b'S') => 8.7,
      (0, b'T') => 5.0,
      (_, b'T') => 6.7,
      (0, b'U') => 2.9,
      (_, b'U') => 3.3,
      (0, b'V') => 1.5,
      (_, b'V') => 1.0,
      (0, b'W') => 2.7,
      (_, b'W') => 0.91,
      (0, b'X') => 0.05,
      (_, b'X') => 0.27,
      (0, b'Y') => 0.36,
      (_, b'Y') => 1.6,
      (0, b'Z') => 0.24,
      (_, b'Z') => 0.44,
      _ => unreachable!(),
    })
    .sum::<f32>();
  if no_repeated_letters(word) {
    score *= 5.0;
  }
  score
}

impl Guesser {
  pub fn new() -> Self {
    let mut candidates = FIVE_LETTER_WORDS.to_vec();
    candidates.sort_by(|a, b| score(b).total_cmp(&score(a)));
    Self {
      candidates,
      excluded: ArrayVec::new(),
      required: ArrayVec::new(),
      confirmed: [const { None }; 5],
    }
  }

  pub fn guess(&self) -> Option<&[u8; 5]> {
    self.candidates.first()
  }

  pub fn candidates(&self) -> &[[u8; 5]] {
    &self.candidates
  }

  pub const fn confirmed_word(&self) -> Option<[u8; 5]> {
    if let [Some(c1), Some(c2), Some(c3), Some(c4), Some(c5)] = self.confirmed {
      Some([c1, c2, c3, c4, c5])
    } else {
      None
    }
  }

  fn confirm(&mut self, idx: usize, ch: u8) {
    self.confirmed[idx] = Some(ch);
    if VERBOSE_MESSAGES {
      println!("letter '{}' is confirmed at position {}", char::from(ch), idx + 1);
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
    assert_ne!(num_possible_positions, 0, "letter '{}' has no possible placement", char::from(ch));
    if VERBOSE_MESSAGES {
      println!("letter '{}' can only be placed in {possible_positions:?}", char::from(ch));
    }
    if num_possible_positions == 1 {
      assert!(!possible_positions.is_empty());
      let only_open = possible_positions.into_index();
      if VERBOSE_MESSAGES {
        println!("letter '{}' can only be placed at position {}", char::from(ch), only_open + 1);
      }
      self.confirm(only_open, ch);
      _ = self.required.remove(idx);
      true
    } else {
      false
    }
  }

  pub fn analyze(&mut self, chars: [(u8, CharStatus); 5]) {
    for (i, (ch, stat)) in chars.into_iter().enumerate() {
      match stat {
        CharStatus::Excluded => {
          if let Err(pos) = self.excluded.binary_search(&ch) {
            self.excluded.insert(pos, ch);
            if VERBOSE_MESSAGES {
              println!("letter '{}' is not in the word", char::from(ch));
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
            println!("letter '{}' is required but cannot be in {:?}", char::from(ch), self.required[idx].1);
          }
          _ = self.pidgeon(idx);
        }

        CharStatus::Confirmed => {
          self.confirm(i, ch);
          if let Ok(i) = self.required.binary_search_by_key(&ch, |(ch, _)| *ch) {
            if VERBOSE_MESSAGES {
              println!("letter '{}' no longer unknown", char::from(ch));
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
    let include = |word: &[u8; 5]| -> bool {
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
