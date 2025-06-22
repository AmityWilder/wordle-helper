use std::cell::RefCell;
use arrayvec::ArrayVec;
use bitflags::bitflags;
use rayon::prelude::*;
use crate::{dictionary::*, play::grade_many, verbose_println, word::{Letter, Word}, OPTIONS};

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
#[repr(u8)]
pub enum LetterFeedback {
  Excluded,
  Required,
  Confirmed,
}

impl std::fmt::Display for LetterFeedback {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LetterFeedback::Excluded => '\u{2B1C}',
      LetterFeedback::Required => '🟨',
      LetterFeedback::Confirmed => '🟩',
    }.fmt(f)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, align(8))]
pub struct WordFeedback([LetterFeedback; 5]);

impl PartialOrd for WordFeedback {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for WordFeedback {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.to_u64().cmp(&other.to_u64())
  }
}

impl std::hash::Hash for WordFeedback {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.to_u64().hash(state);
  }
}

impl std::ops::Deref for WordFeedback {
  type Target = [LetterFeedback; 5];

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

impl WordFeedback {
  pub const COMBINATIONS: usize = 3usize.pow(5);

  #[inline(always)]
  pub const fn new(values: [LetterFeedback; 5]) -> Self {
    Self(values)
  }

  #[inline(always)]
  pub const fn to_u64(self) -> u64 {
    unsafe { std::mem::transmute::<_, u64>(self) }
  }
}

struct FeedbackMap<T> {
  data: Vec<(WordFeedback, T)>,
}

impl<T> FeedbackMap<T> {
  pub fn with_capacity(capacity: usize) -> Self {
    Self {
      data: Vec::with_capacity(capacity),
    }
  }

  pub const fn len(&self) -> usize {
    self.data.len()
  }

  pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, key: WordFeedback, f: F) -> &mut T {
    match self.data.binary_search_by_key(&key, |(k, _)| *k) {
      Ok(idx) => &mut self.data[idx].1,
      Err(idx) => {
        self.data.insert(idx, (key, f()));
        &mut self.data[idx].1
      }
    }
  }

  pub fn values<'a>(&'a self) -> std::iter::Map<std::slice::Iter<'a, (WordFeedback, T)>, fn(&'a (WordFeedback, T)) -> &'a T> {
    self.data.iter().map(|x| &x.1)
  }

  pub fn entries(&self) -> std::slice::Iter<'_, (WordFeedback, T)> {
    self.data.iter()
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

thread_local! {
  static BUFFER: RefCell<Vec<WordFeedback>> = RefCell::new(
    Vec::with_capacity(FIVE_LETTER_WORDS.len()*FIVE_LETTER_WORDS.len())
  );

  static TIEBREAKERS: RefCell<Vec<(Word, FeedbackMap<Vec<Word>>)>> = RefCell::new(
    Vec::with_capacity(FIVE_LETTER_WORDS.len()),
  );
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

  pub fn guess(&self) -> Option<&Word> {
    self.candidates.first()
  }

  pub fn candidates(&self) -> &[Word] {
    &self.candidates
  }

  fn confirm(&mut self, idx: usize, ch: Letter) {
    self.confirmed[idx] = Some(ch);
    verbose_println!("letter '{ch}' is confirmed at position {}", idx + 1);
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
    verbose_println!("letter '{ch}' can only be placed in {possible_positions:?}");
    if num_possible_positions == 1 {
      assert!(!possible_positions.is_empty());
      let only_open = possible_positions.into_index();
      verbose_println!("letter '{ch}' can only be placed at position {}", only_open + 1);
      self.confirm(only_open, ch);
      _ = self.required.remove(idx);
      true
    } else {
      false
    }
  }

  pub fn analyze(&mut self, chars: [(Letter, LetterFeedback); 5]) {
    if !matches!(chars, [
      (_, LetterFeedback::Confirmed),
      (_, LetterFeedback::Confirmed),
      (_, LetterFeedback::Confirmed),
      (_, LetterFeedback::Confirmed),
      (_, LetterFeedback::Confirmed),
    ]) {
      let word_used = Word(chars.map(|(c, _)| c));
      if let Some(pos) = self.candidates.iter().position(|word| word == &word_used) {
        _ = self.candidates.remove(pos);
      } // else: user-provided word
    }

    for (i, (ch, stat)) in chars.into_iter().enumerate() {
      match stat {
        LetterFeedback::Excluded => {
          if let Err(pos) = self.excluded.binary_search(&ch) {
            self.excluded.insert(pos, ch);
            verbose_println!("letter '{ch}' is not in the word");
          }
        }

        LetterFeedback::Required => {
          let pos = Positions::from_index(i).unwrap();
          let idx = match self.required.binary_search_by_key(&ch, |(r, _)| *r) {
            Ok(idx) => { self.required[idx].1.insert(pos); idx },
            Err(idx) => { self.required.insert(idx, (ch, pos)); idx },
          };
          verbose_println!("letter '{ch}' is required but cannot be in {:?}", self.required[idx].1);
          _ = self.pidgeon(idx);
        }

        LetterFeedback::Confirmed => {
          self.confirm(i, ch);
          if let Ok(i) = self.required.binary_search_by_key(&ch, |(ch, _)| *ch) {
            verbose_println!("letter '{ch}' no longer unknown");
            _ = self.required.remove(i);
          }
        }
      }
    }

    verbose_println!("draining...");
    'outer: loop {
      for i in 0..self.required.len() {
        if self.pidgeon(i) {
          continue 'outer;
        }
      }
      break;
    }
    verbose_println!("feedback complete");
  }

  #[inline(never)]
  fn encode_burner(&self) -> Option<Word> {
    TIEBREAKERS.with_borrow_mut(|possible_tiebreakers| {
      possible_tiebreakers.clear();

      BUFFER.with_borrow_mut(|buf| {
        // Pretend the candidate IS the actual word.
        // If that were the case, how would our tiebreaker be judged?
        buf.clear();
        buf.par_extend(grade_many(FIVE_LETTER_WORDS.as_slice(), self.candidates.as_slice()).map(|(_, _, x)| x));

        for (i, guess) in FIVE_LETTER_WORDS.iter().copied().enumerate() {
          let mut mapping = FeedbackMap::with_capacity(8);
          for (j, word) in self.candidates.iter().copied().enumerate() {
            let encoding = buf[i * self.candidates.len() + j];
            mapping.get_or_insert_with(encoding, || Vec::with_capacity(8))
              .push(word);
          }
          possible_tiebreakers.push((guess, mapping));
        }
      });

      // don't bother if the burner would have been just as effective as trying both
      possible_tiebreakers.retain(|(_, mapping)| mapping.len() > 2);

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

      let mut om_buf = Vec::with_capacity(self.candidates.len());
      om_buf.par_extend(grade_many(&self.candidates[0..1], self.candidates.as_slice()).map(|(_, _, x)| x));

      let mut it = om_buf.into_iter();
      let mut organic_mappings = FeedbackMap::with_capacity(8);
      for word in self.candidates.iter().copied() {
        let encoding = it.next().unwrap();
        organic_mappings.get_or_insert_with(encoding, || Vec::with_capacity(8))
          .push(word);
      }
      let organic_mappings = (self.candidates[0], organic_mappings);

      if OPTIONS.get().unwrap().is_verbose {
        fn tiebreaker_printout((word, mapping): &(Word, FeedbackMap<Vec<Word>>)) {
          println!(" {word}");
          for (encoding, words) in mapping.entries() {
            print!("  {encoding} -");
            for w in words {
              print!(" {w}");
            }
            println!();
          }
        }

        println!("upcoming organic guess:");
        tiebreaker_printout(&organic_mappings);
        println!("possible tiebreakers:");
        for tb in possible_tiebreakers.iter().take(5) {
          tiebreaker_printout(tb);
        }
      }

      let possible_tiebreakers = possible_tiebreakers.iter();

      let (_, organic_mappings) = organic_mappings;

      possible_tiebreakers
        // only check the best tiebreaker candidates
        .take(5)
        // compare the narrowing of the tiebreaker to that of the first candidate.
        // only use a tiebreaker if guaranteed to actually provide an advantage
        .find_map(|(tiebreaker, mapping)| {
          use std::cmp::Ordering;
          match mapping.len().cmp(&organic_mappings.len()) {
            // fewer buckets than organic; guaranteed less potent
            Ordering::Less => false,

            // more buckets than organic; guaranteed more potent
            Ordering::Greater => true,

            // compare potency
            Ordering::Equal => {
              match mapping.values().map(|v| v.len()).max().cmp(&organic_mappings.values().map(|v| v.len()).max()) {
                // worst case has better chance than for organic
                Ordering::Less => true,

                // worst case has worse chance than for organic
                Ordering::Greater => false,

                // last chance to prove yourself:
                // same number of buckets, same worst case, who has a better average case?
                // (don't need to divide because denominator is shared)
                Ordering::Equal => mapping.values().map(|v| v.len()).sum::<usize>() < organic_mappings.values().map(|v| v.len()).sum::<usize>(),
              }
            }
          }.then_some(*tiebreaker)
        })
    })
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

    if turn < 6 && matches!(self.candidates.len(), 3..=26) { // WordFeedback::COMBINATIONS
      if let Some(tiebreaker) = self.encode_burner() {
        verbose_println!("tiebreaker: {tiebreaker}");
        self.candidates.insert(0, tiebreaker);
      }
    }
  }
}
