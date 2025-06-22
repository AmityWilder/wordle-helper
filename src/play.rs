use rayon::iter::{IntoParallelIterator, ParallelIterator};
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

pub fn grade_many(guesses: &[Word], words: &[Word]) -> rayon::iter::Map<rayon::range::Iter<usize>, impl Fn(usize) -> (Word, Word, WordFeedback)> {
  let words_len = words.len();
  (0..guesses.len()*words_len)
    .into_par_iter()
    .map(move |i| {
      let (guess, word) = (guesses[i / words_len], words[i % words_len]);
      (guess, word, check_word(word, guess))
    })
}
