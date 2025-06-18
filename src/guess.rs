use arrayvec::ArrayVec;
use bitflags::bitflags;
use crate::dictionary::*;

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

impl Guesser {
  pub fn new() -> Self {
    let mut candidates = FIVE_LETTER_WORDS.to_vec();
    candidates.sort_by_key(|word| {
      let mut score = 0;
      score += word.iter()
        .enumerate()
        .map(|item| match item {
          (_, b'A') => 2,
          (_, b'E') => 2,
          (_, b'I') => 1,
          (_, b'O') => 1,
          (_, b'U') => 1,
          (_, b'Q') => -1,
          (5, b'Y') => 1,
          (_, b'Y') => -1,
          (_, b'W') => -1,
          (_, b'Z') => -2,
          _ => 0,
        })
        .sum::<i8>();
      if no_repeated_letters(word) {
        score *= 3;
      }
      -score
    });
    Self {
      candidates,
      excluded: ArrayVec::new(),
      required: ArrayVec::new(),
      confirmed: [const { None }; 5],
    }
  }

  pub fn suggestion(&self) -> Option<&[u8; 5]> {
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

  fn confirmed_positions(&self) -> Positions {
    Positions::from_iter(
      self.confirmed.iter()
        .enumerate()
        .filter(|(_, ch)| ch.is_some())
        .map(|(i, _)| Positions::from_index(i).unwrap())
    )
  }

  fn confirm(&mut self, idx: usize, ch: u8) {
    self.confirmed[idx] = Some(ch);
    #[cfg(debug_assertions)]
    println!("letter '{}' is confirmed at position {}", char::from(ch), idx + 1);
  }

  /// If only one possible space, treat as confirmed
  ///
  /// Returns `true` if an unknown was confirmed
  fn pidgeon(&mut self, idx: usize) -> bool {
    let (ch, p) = self.required[idx];
    let possible_positions = p
      .union(self.confirmed_positions())
      .complement();
    let num_possible_positions = possible_positions.bits().count_ones();
    assert_ne!(num_possible_positions, 0, "letter '{}' has no possible placement", char::from(ch));
    #[cfg(debug_assertions)]
    println!("letter '{}' can only be placed in {possible_positions:?}", char::from(ch));
    if num_possible_positions == 1 {
      assert!(!possible_positions.is_empty());
      let only_open = possible_positions.into_index();
      #[cfg(debug_assertions)]
      println!("letter '{}' can only be placed at position {}", char::from(ch), only_open + 1);
      self.confirm(only_open, ch);
      _ = self.required.remove(idx);
      true
    } else {
      false
    }
  }

  pub fn submit_feedback(&mut self, chars: [(u8, CharStatus); 5]) {
    for (i, (ch, stat)) in chars.into_iter().enumerate() {
      match stat {
        CharStatus::Excluded => {
          if let Err(pos) = self.excluded.binary_search(&ch) {
            self.excluded.insert(pos, ch);
            #[cfg(debug_assertions)]
            println!("letter '{}' is not in the word", char::from(ch));
          }
        }

        CharStatus::Required => {
          let pos = Positions::from_index(i).unwrap();
          let idx = match self.required.binary_search_by_key(&ch, |(r, _)| *r) {
            Ok(idx) => { self.required[idx].1.insert(pos); idx },
            Err(idx) => { self.required.insert(idx, (ch, pos)); idx },
          };
          #[cfg(debug_assertions)]
          println!("letter '{}' is required but cannot be in {:?}", char::from(ch), self.required[idx].1);
          _ = self.pidgeon(idx);
        }

        CharStatus::Confirmed => {
          self.confirm(i, ch);
          if let Ok(i) = self.required.binary_search_by_key(&ch, |(ch, _)| *ch) {
            #[cfg(debug_assertions)]
            println!("letter '{}' no longer unknown", char::from(ch));
            _ = self.required.remove(i);
          }
        }
      }
    }

    #[cfg(debug_assertions)]
    println!("draining...");
    'outer: loop {
      for i in 0..self.required.len() {
        if self.pidgeon(i) {
          continue 'outer;
        }
      }
      break;
    }
    #[cfg(debug_assertions)]
    println!("feedback complete");
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
