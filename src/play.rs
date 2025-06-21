use std::simd::{cmp::SimdPartialEq, Simd};

use crate::{guess::{LetterFeedback, WordFeedback}, word::Word};

pub fn check_word(word: Word, guess: Word) -> WordFeedback {
  WordFeedback::new(std::array::from_fn(|i|
    if word.0.contains(&guess.0[i]) {
      if word.0[i] == guess.0[i] {
        LetterFeedback::Confirmed
      } else {
        LetterFeedback::Required
      }
    } else {
      LetterFeedback::Excluded
    }
  ))
}

pub fn grade_many(guesses: &[Word], words: &[Word], feedback_buf: &mut [WordFeedback]) {
  const _: () = {
    assert!(LetterFeedback::Excluded as u8 == 0);
    assert!(LetterFeedback::Required as u8 == 1);
    assert!(LetterFeedback::Confirmed as u8 == 2);
  };
  let (gs, ws) = unsafe { (
    std::mem::transmute::<&[Word], &[[u8; 5]]>(guesses),
    std::mem::transmute::<&[Word], &[[u8; 5]]>(words),
  ) };
  assert_eq!(feedback_buf.len(), gs.len()*ws.len());
  let mut k = 0;
  for &[g0, g1, g2, g3, g4] in gs {
    let g0 = Simd::<u8, 5>::splat(g0);
    let g1 = Simd::<u8, 5>::splat(g1);
    let g2 = Simd::<u8, 5>::splat(g2);
    let g3 = Simd::<u8, 5>::splat(g3);
    let g4 = Simd::<u8, 5>::splat(g4);
    for &w in ws {
      let w = Simd::<u8, 5>::from_array(w);
      let e0 = w.simd_eq(g0);
      let e1 = w.simd_eq(g1);
      let e2 = w.simd_eq(g2);
      let e3 = w.simd_eq(g3);
      let e4 = w.simd_eq(g4);
      let arr = [
        (e0.test(0) as u8*0b11)^(e0.any() as u8),
        (e1.test(1) as u8*0b11)^(e1.any() as u8),
        (e2.test(2) as u8*0b11)^(e2.any() as u8),
        (e3.test(3) as u8*0b11)^(e3.any() as u8),
        (e4.test(4) as u8*0b11)^(e4.any() as u8),
      ];
      feedback_buf[k] = WordFeedback::new(unsafe { std::mem::transmute::<[u8; 5], [LetterFeedback; 5]>(arr) });
      k += 1;
    }
  }
}
