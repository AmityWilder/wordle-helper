use std::sync::LazyLock;
use crate::word::Word;

const UNSORTED_FIVE_LETTER_WORDS: [Word; 12915] = unsafe { std::mem::transmute(include!("list.rs")) };

pub fn sort_by_frequency(words: &mut [Word]) {
  let mut freq_analysis = [[0; 26]; 5];
  for word in &*words {
    for (ch, freq) in word.into_iter().zip(freq_analysis.iter_mut()) {
      freq[ch.index()] += 1;
    }
  }

  words.sort_by_cached_key(|word|
    u32::MAX - word.iter()
      .copied()
      .enumerate()
      .map(|(i, ch)| freq_analysis[i][ch.index()])
      .sum::<u32>()
  );

  // partition unique words to the front
  words.sort_by_cached_key(|word| !word.is_unique());
}

pub static FIVE_LETTER_WORDS: LazyLock<Vec<Word>> = LazyLock::new(|| {
  let mut words = UNSORTED_FIVE_LETTER_WORDS.to_vec();
  sort_by_frequency(&mut words);
  words
});
