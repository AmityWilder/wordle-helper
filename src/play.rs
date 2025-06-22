use std::{num::NonZero, sync::Mutex};
use arrayvec::ArrayVec;
use crate::{guess::{LetterFeedback, WordFeedback}, word::Word};

pub fn check_word(word: Word, guess: Word) -> WordFeedback {
  WordFeedback::new(std::array::from_fn(|i|
    if word.0[i] == guess.0[i] {
      LetterFeedback::Confirmed
    } else if word.0.contains(&guess.0[i]) {
      LetterFeedback::Required
    } else {
      LetterFeedback::Excluded
    }
  ))
}

pub struct CartesianProduct<A: Iterator, B> {
  a: A,
  b: B,
  curr: Option<(A::Item, B)>,
}

impl<A: Iterator, B> CartesianProduct<A, B> {
  fn new(a: A, b: B) -> Self {
    Self { a, b, curr: None }
  }
}

impl<A: Iterator<Item: Clone>, B: Iterator + Clone> Iterator for CartesianProduct<A, B> {
  type Item = (A::Item, B::Item);

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.curr.is_none() {
        self.curr = self.a.next()
          .map(|a| (a, self.b.clone()));
      }

      if let Some((ref a, ref mut b)) = self.curr {
        if let Some(b) = b.next() {
          break Some((a.clone(), b));
        } else {
          self.curr = None;
        }
      } else {
        break None;
      }
    }
  }
}

impl<A: ExactSizeIterator, B: ExactSizeIterator> ExactSizeIterator for CartesianProduct<A, B> where Self: Iterator {
  #[inline]
  fn len(&self) -> usize {
    self.a.len() * self.b.len() + match &self.curr {
      Some((_, b)) => b.len(),
      None => 0,
    }
  }
}

impl<A: Iterator<Item: Clone> + Clone, B: Clone + Iterator> Clone for CartesianProduct<A, B> {
  #[inline]
  fn clone(&self) -> Self {
    Self {
      a: self.a.clone(),
      b: self.b.clone(),
      curr: self.curr.clone(),
    }
  }
}
impl<A: Iterator<Item: Copy> + Copy, B: Copy + Iterator> Copy for CartesianProduct<A, B> {}

impl<A: Iterator, B> std::iter::FusedIterator for CartesianProduct<A, B> where Self: Iterator {}

#[inline]
pub fn cartesian_prod<A, B>(a: A, b: B) -> CartesianProduct<A::IntoIter, B::IntoIter>
where
  A: IntoIterator,
  B: IntoIterator,
{
  CartesianProduct::new(a.into_iter(), b.into_iter())
}

pub trait CartesianProductExt: Iterator {
  #[inline]
  fn cartesian_prod<U>(self, other: U) -> CartesianProduct<Self, U::IntoIter>
  where
    Self: Sized,
    U: IntoIterator,
  {
    cartesian_prod(self.into_iter(), other)
  }
}

impl<I: Iterator> CartesianProductExt for I {}

pub fn grade_many<'g, 'w>(guesses: &'g [Word], words: &'w [Word], buffer: &mut [WordFeedback]) {
  const ONE: NonZero<usize> = unsafe { NonZero::<usize>::new_unchecked(1) };

  assert_eq!(buffer.len(), guesses.len()*words.len());

  let work = guesses.iter().copied().cartesian_prod(words.iter().copied()).zip(buffer);

  let n = std::thread::available_parallelism().unwrap_or(ONE);
  if n == ONE {
    // just use the current thread
    for ((guess, word), buf) in work {
      *buf = check_word(word, guess);
    }
  } else {
    const CHUNK_SIZE: usize = 256;
    let work = Mutex::new(work);
    std::thread::scope(|s| {
      const MAX_THREADS: usize = 256;
      let mut threads = ArrayVec::<_, MAX_THREADS>::new();
      for _ in 0..n.get() {
        let thread = s.spawn(||
          loop {
            let group = match work.lock().unwrap().next_chunk::<CHUNK_SIZE>() {
              Ok(x) => x.into_iter(),
              Err(x) => if x.len() > 0 { x.into_iter() } else { return; }
            };
            for ((guess, word), buf) in group {
              *buf = check_word(word, guess);
            }
          }
        );
        threads.push(thread);
      }
    });
  }
}
