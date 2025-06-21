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
  let (guesses, words) = unsafe { (
    std::mem::transmute::<&[Word], &[[u8; 5]]>(guesses),
    std::mem::transmute::<&[Word], &[[u8; 5]]>(words),
  ) };
  assert_eq!(feedback_buf.len(), guesses.len()*words.len());
  let mut k = 0;
  for i in 0..guesses.len() {
    let guess = unsafe { guesses.get_unchecked(i) };
    for j in 0..words.len() {
      let word = unsafe { words.get_unchecked(j) };
      unsafe {
        *feedback_buf.get_unchecked_mut(k) = WordFeedback::new(
          std::mem::transmute::<[u8; 5], [LetterFeedback; 5]>([
            ((guess[0] == word[0]) as u8*3)^((guess[0] == word[0]) as u8 | (guess[0] == word[1]) as u8 | (guess[0] == word[2]) as u8 | (guess[0] == word[3]) as u8 | (guess[0] == word[4]) as u8),
            ((guess[1] == word[1]) as u8*3)^((guess[1] == word[0]) as u8 | (guess[1] == word[1]) as u8 | (guess[1] == word[2]) as u8 | (guess[1] == word[3]) as u8 | (guess[1] == word[4]) as u8),
            ((guess[2] == word[2]) as u8*3)^((guess[2] == word[0]) as u8 | (guess[2] == word[1]) as u8 | (guess[2] == word[2]) as u8 | (guess[2] == word[3]) as u8 | (guess[2] == word[4]) as u8),
            ((guess[3] == word[3]) as u8*3)^((guess[3] == word[0]) as u8 | (guess[3] == word[1]) as u8 | (guess[3] == word[2]) as u8 | (guess[3] == word[3]) as u8 | (guess[3] == word[4]) as u8),
            ((guess[4] == word[4]) as u8*3)^((guess[4] == word[0]) as u8 | (guess[4] == word[1]) as u8 | (guess[4] == word[2]) as u8 | (guess[4] == word[3]) as u8 | (guess[4] == word[4]) as u8),
          ])
        );
      }
      k += 1;
    }
  }
}
