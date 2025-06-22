use std::num::NonZero;
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

pub fn grade_many(guesses: &[Word], words: &[Word], feedback_buf: &mut [WordFeedback]) {
  assert_eq!(feedback_buf.len(), guesses.len()*words.len());
  let gs = unsafe { std::mem::transmute::<&[Word], &[[u8; 5]]>(guesses) };
  let ws = unsafe { std::mem::transmute::<&[Word], &[[u8; 5]]>(words) };
  let fb = feedback_buf;

  const USE_MULTITHREADING: bool = true;
  if USE_MULTITHREADING {
    let num_threads = std::thread::available_parallelism()
      .map(|n| n.min(unsafe { NonZero::new_unchecked(64) }))
      .unwrap_or(const { unsafe { NonZero::<usize>::new_unchecked(1) } });

    std::thread::scope(|s| {
      let _threads = gs.chunks((gs.len()/num_threads).max(1))
        .zip(ws.chunks((ws.len()/num_threads).max(1)))
        .zip(fb.chunks_mut((fb.len()/num_threads).max(1)))
        .map(|((gs, ws), fb)| {
        s.spawn(|| {
          for ((w, g), fb) in gs.iter().flat_map(|g| ws.iter().zip(std::iter::repeat(g))).zip(fb.iter_mut()) {
            *fb = WordFeedback::new(std::array::from_fn(|i|
              if w[i] == g[i] {
                LetterFeedback::Confirmed
              } else if w.contains(&g[i]) {
                LetterFeedback::Required
              } else {
                LetterFeedback::Excluded
              }
            ));
          }
        })
      }).collect::<ArrayVec<_, 64>>();
    });
  } else {
    for ((w, g), fb) in gs.iter().flat_map(|g| ws.iter().zip(std::iter::repeat(g))).zip(fb.iter_mut()) {
      *fb = WordFeedback::new(std::array::from_fn(|i|
        if w[i] == g[i] {
          LetterFeedback::Confirmed
        } else if w.contains(&g[i]) {
          LetterFeedback::Required
        } else {
          LetterFeedback::Excluded
        }
      ));
    }
  }
}
