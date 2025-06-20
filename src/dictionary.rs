use std::sync::LazyLock;
use crate::word::Word;

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
  let mut words = include_bytes!("list.txt")
    .split(|&ch| ch == b';')
    .map(|word| {
      debug_assert_eq!(word.len(), 5);
      let bytes = unsafe { *(word.as_ptr() as *const [u8; 5]) };
      #[cfg(debug_assertions)] {
        Word::from_bytes(bytes).expect("words in list.txt should be valid")
      }
      #[cfg(not(debug_assertions))] {
        unsafe { Word::from_bytes_unchecked(bytes) }
      }
    })
    .collect::<Vec<Word>>();
  sort_by_frequency(&mut words);
  words
});
