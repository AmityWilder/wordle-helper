use std::collections::HashMap;

use arrayvec::ArrayVec;
use bitflags::bitflags;
use rand::seq::IteratorRandom;
use crate::{dictionary::*, play::Game, word::{Letter, Word}, VERBOSE_MESSAGES};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharFeedback {
  Excluded,
  Required,
  Confirmed,
}

impl std::fmt::Display for CharFeedback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CharFeedback::Excluded => '\u{2B1C}',
      CharFeedback::Required => '🟨',
      CharFeedback::Confirmed => '🟩',
    }.fmt(f)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WordFeedback(pub [CharFeedback; 5]);

impl std::ops::Deref for WordFeedback {
  type Target = [CharFeedback; 5];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl std::ops::DerefMut for WordFeedback {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl std::fmt::Display for WordFeedback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for ch in self.0 {
      ch.fmt(f)?;
    }
    Ok(())
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

  pub fn extract_resources(self) -> Vec<Word> {
    self.candidates
  }

  pub fn guess<R: ?Sized + rand::Rng>(&self, turn: u32, rng: &mut R) -> Option<&Word> {
    if turn == 1 {
      self.candidates.iter()
        .take(8)
        .choose(rng)
    } else {
      self.candidates.first()
    }
  }

  pub fn candidates(&self) -> &[Word] {
    &self.candidates
  }

  fn confirm(&mut self, idx: usize, ch: Letter) {
    self.confirmed[idx] = Some(ch);
    if *VERBOSE_MESSAGES {
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
    if *VERBOSE_MESSAGES {
      println!("letter '{ch}' can only be placed in {possible_positions:?}");
    }
    if num_possible_positions == 1 {
      assert!(!possible_positions.is_empty());
      let only_open = possible_positions.into_index();
      if *VERBOSE_MESSAGES {
        println!("letter '{ch}' can only be placed at position {}", only_open + 1);
      }
      self.confirm(only_open, ch);
      _ = self.required.remove(idx);
      true
    } else {
      false
    }
  }

  pub fn analyze(&mut self, chars: [(Letter, CharFeedback); 5]) {
    if !matches!(chars, [
      (_, CharFeedback::Confirmed),
      (_, CharFeedback::Confirmed),
      (_, CharFeedback::Confirmed),
      (_, CharFeedback::Confirmed),
      (_, CharFeedback::Confirmed),
    ]) {
      let word_used = Word(chars.map(|(c, _)| c));
      if let Some(pos) = self.candidates.iter().position(|word| word == &word_used) {
        _ = self.candidates.remove(pos);
      } // else: user-provided word
    }

    for (i, (ch, stat)) in chars.into_iter().enumerate() {
      match stat {
        CharFeedback::Excluded => {
          if let Err(pos) = self.excluded.binary_search(&ch) {
            self.excluded.insert(pos, ch);
            if *VERBOSE_MESSAGES {
              println!("letter '{ch}' is not in the word");
            }
          }
        }

        CharFeedback::Required => {
          let pos = Positions::from_index(i).unwrap();
          let idx = match self.required.binary_search_by_key(&ch, |(r, _)| *r) {
            Ok(idx) => { self.required[idx].1.insert(pos); idx },
            Err(idx) => { self.required.insert(idx, (ch, pos)); idx },
          };
          if *VERBOSE_MESSAGES {
            println!("letter '{ch}' is required but cannot be in {:?}", self.required[idx].1);
          }
          _ = self.pidgeon(idx);
        }

        CharFeedback::Confirmed => {
          self.confirm(i, ch);
          if let Ok(i) = self.required.binary_search_by_key(&ch, |(ch, _)| *ch) {
            if *VERBOSE_MESSAGES {
              println!("letter '{ch}' no longer unknown");
            }
            _ = self.required.remove(i);
          }
        }
      }
    }

    if *VERBOSE_MESSAGES {
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
    if *VERBOSE_MESSAGES {
      println!("feedback complete");
    }
  }

  fn encode_burner(&self) -> Option<Word> {
    fn generate_mapping(tiebreaker: &Word, candidates: &[Word]) -> Option<(Word, HashMap<WordFeedback, Vec<Word>>)> {
      let mut mapping = HashMap::new();
      for candidate in candidates {
        // Pretend the candidate IS the actual word.
        // If that were the case, how would our tiebreaker be judged?
        let encoding = Game::new(*candidate)
          .check(&tiebreaker);

        mapping.entry(encoding)
          .and_modify(|v: &mut Vec<Word>| v.push(*candidate))
          .or_insert_with(|| vec![*candidate]);
      }
      // don't bother if the burner would have been just as effective as trying both
      if mapping.len() > 2 {
        Some((*tiebreaker, mapping))
      } else {
        None
      }
    }

    let mut possible_tiebreakers = FIVE_LETTER_WORDS.iter()
      .filter_map(|tiebreaker| generate_mapping(tiebreaker, &self.candidates))
      .collect::<Vec<_>>();

    // prefer words with fewer letters we already know
    possible_tiebreakers.sort_by_cached_key(|(w, _)|
      self.excluded.iter().copied()
        .chain(self.required.iter().copied().map(|(ch, _)| ch))
        .chain(self.confirmed.iter().copied().filter_map(|ch| ch))
        .filter(|ch| w.contains(ch))
        .count()
    );

    // prefer words with more tiebreakers
    possible_tiebreakers.sort_by_key(|(_, m)| usize::MAX - m.len());

    // prefer more potent tiebreakers
    possible_tiebreakers.sort_by_key(|(_, m)|
      m.values()
        // more words in the same bucket are exponentially less valuable than having the same number of words in more buckets
        .map(|v| v.len().saturating_pow(4))
        .sum::<usize>()
    );

    // prefer words without repeated letters
    possible_tiebreakers.sort_by_cached_key(|(w, _)| !w.is_unique());

    if *VERBOSE_MESSAGES {
      println!("possible tiebreakers:");
      for (word, mapping) in &possible_tiebreakers {
        println!(" {word}");
        for (encoding, words) in mapping {
          print!("  {encoding} -");
          for w in words {
            print!(" {w}");
          }
          println!();
        }
      }
    }

    let possible_tiebreakers = possible_tiebreakers.into_iter();

    if let Some((_, organic_mapping)) = generate_mapping(&self.candidates[0], &self.candidates) {
      possible_tiebreakers
        // only check the best tiebreaker candidates
        .take(5)
        // compare the narrowing of the tiebreaker to that of the first candidate.
        // only use a tiebreaker if guaranteed to actually provide an advantage
        .find_map(|(tiebreaker, mapping)| {
          use std::cmp::Ordering;
          match mapping.len().cmp(&organic_mapping.len()) {
            // fewer buckets than organic; guaranteed less potent
            Ordering::Less => false,

            // more buckets than organic; guaranteed more potent
            Ordering::Greater => true,

            // compare potency
            Ordering::Equal => {
              match mapping.values().map(|v| v.len()).max().cmp(&organic_mapping.values().map(|v| v.len()).max()) {
                // worst case has better chance than for organic
                Ordering::Less => true,

                // worst case has worse chance than for organic
                Ordering::Greater => false,

                // last chance to prove yourself:
                // same number of buckets, same worst case, who has a better average case?
                // (don't need to divide because denominator is shared)
                Ordering::Equal => mapping.values().map(|v| v.len()).sum::<usize>() < organic_mapping.values().map(|v| v.len()).sum::<usize>(),
              }
            }
          }.then_some(tiebreaker)
        })
    } else {
      // organic wasn't even worth it but the tiebreaker is
      possible_tiebreakers
        .map(|(w, _)| w)
        .next()
    }
  }

  pub fn prune(&mut self, turn: u32) {
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
    sort_by_frequency(&mut self.candidates);

    if turn < 6 && matches!(self.candidates.len(), 3..=26) {
      if let Some(tiebreaker) = self.encode_burner() {
        if *VERBOSE_MESSAGES {
          println!("tiebreaker: {tiebreaker}");
        }
        self.candidates.insert(0, tiebreaker);
      }
    }
  }
}
