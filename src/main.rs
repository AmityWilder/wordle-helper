#![feature(portable_simd)]
use std::{io::stdin, num::NonZeroUsize, sync::LazyLock};
use arrayvec::ArrayVec;
use guess::*;
use crate::{dictionary::FIVE_LETTER_WORDS, play::check_word, word::{Letter, Word}};

mod word;
mod dictionary;
mod guess;
mod play;

pub struct AppOptions {
  /// Print excessive debug information about the strategy's "thought process" while it plays
  pub is_verbose: bool,

  /// Every confirmed letter MUST be used in all subsequent guesses
  pub is_hardmode: bool,

  /// Play a specified number of games and generate stats on wins/losses/speed
  ///
  /// NOTE: Incompatible with `-v` for performance reasons and `-a` due to mutual incompatibility
  pub stats_run: Option<NonZeroUsize>,

  /// Provide the winning word and see how the application tries to solve it
  pub auto_run: Option<Word>,
}

impl AppOptions {
  pub fn init() -> Self {
    let mut is_verbose = false;
    let mut is_hardmode = false;
    let mut stats_run = None;
    let mut auto_run = None;

    for arg in std::env::args() {
      println!("{arg}");
      if arg == "-v" {
        is_verbose = true;
      } else if arg == "-h" {
        is_hardmode = true;
      } else {
        if let Some(arg) = arg.strip_prefix("-s") {
          if !arg.is_empty() {
            stats_run = arg.parse().ok().and_then(NonZeroUsize::new);
          } else {
            stats_run = Some(NonZeroUsize::new(FIVE_LETTER_WORDS.len())
              .expect("should have at least 1 word in dictionary"))
          }
          if stats_run.is_some() {
            if auto_run.is_some() {
              println!("autorun is incompatible with stats run; disabling autorun");
              auto_run = None;
            }
          } else {
            println!("warning: stats run argument malformed. stats run will not be enabled");
          }
        } else if let Some(arg) = arg.strip_prefix("-a") {
          if let [a, b, c, d, e] = arg.as_bytes() {
            auto_run = Word::from_bytes([a, b, c, d, e].map(|ch| ch.to_ascii_uppercase()));
          }
          if auto_run.is_some() {
            if stats_run.is_some() {
              println!("stats run is incompatible with autorun; disabling stats run");
              stats_run = None;
            }
          } else {
            println!("warning: autorun argument malformed. autorun will not be enabled");
          }
        }
      }
    }

    if stats_run.is_some() {
      if is_verbose {
        println!("verbose messages are disabled for stats runs");
        is_verbose = false;
      }
    }

    Self {
      is_verbose,
      is_hardmode,
      stats_run,
      auto_run,
    }
  }
}

pub static OPTIONS: LazyLock<AppOptions> = LazyLock::new(AppOptions::init);

pub struct Attempts(ArrayVec::<WordFeedback, 6>);

impl Attempts {
  pub const fn new() -> Self {
    Self(ArrayVec::new_const())
  }

  pub fn push(&mut self, stats: WordFeedback) {
    self.0.push(stats);
  }
}

impl std::fmt::Display for Attempts {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for row in 0..self.0.len() {
      for col in &*self.0[row] {
        col.fmt(f)?;
      }
      if row + 1 < self.0.len() {
        '\n'.fmt(f)?;
      }
    }
    Ok(())
  }
}

fn main() {
  if let Some(n) = OPTIONS.stats_run {
    statistics(n);
  } else {
    let game_word = OPTIONS.auto_run;
    let mut buf = String::with_capacity(12);
    let mut guesser = Guesser::new(Vec::new());
    let mut attempts = Attempts::new();

    for turn in 1..=6 {
      println!("turn {turn} ({} remaining):", 6 - turn);
      let Some(s) = guesser.guess() else {
        println!("no such word exists in my dictionary");
        return;
      };
      println!("suggestion: {s}");
      let feedback = if let Some(g) = &game_word {
        let fb = check_word(*g, *s);
        std::array::from_fn(|i| (s[i], fb[i]))
      } else {
        buf.clear();
        stdin().read_line(&mut buf).unwrap();
        buf.truncate(buf.trim_end().len());
        if buf.trim_end() == "exit" { return; }
        stdin().read_line(&mut buf).unwrap();
        buf.truncate(buf.trim_end().len());
        assert!(buf.len() == 10);
        let bytes = buf.as_bytes();
        std::array::from_fn(|i| (
          Letter::from_u8(bytes[i].to_ascii_uppercase())
            .expect("unknown format"),
          match bytes[i + 5] {
            b'+' => LetterFeedback::Confirmed,
            b'?' => LetterFeedback::Required,
            b'_' => LetterFeedback::Excluded,
            _ => panic!("unknown format"),
          },
        ))
      };
      attempts.push(WordFeedback::new(feedback.map(|(_, stat)| stat)));
      if attempts.0.last() == Some(&WordFeedback::new([LetterFeedback::Confirmed; 5])) {
        println!("{attempts}");
        let word = Word(feedback.map(|(ch, _)| ch));
        println!("success! winning word: {word}");
        return;
      }
      guesser.analyze(feedback);
      guesser.prune(turn);
      print!("candidates:");
      for (n, word) in (0..7).cycle().zip(guesser.candidates()) {
        if n == 0 { println!(); }
        print!("{word} ");
      }
      println!();
      println!("{attempts}");
    }
    println!("game over");
  }
}

pub fn statistics(n: NonZeroUsize) {
  const BATCH_SIZE: usize = 100;
  let mut candidates_buf = Some(Vec::new());
  let mut games: Vec<(bool, Word, ArrayVec<Word, 6>)> = Vec::with_capacity(n.get());
  let mut batch = 0;
  'rounds: for (cycle, word) in (0..BATCH_SIZE).cycle().zip(FIVE_LETTER_WORDS.iter()) {
    if cycle == 0 {
      println!("{:3.3}% complete", 100.0*batch as f64/n.get() as f64);
      batch += BATCH_SIZE;
    }
    let mut guesser = Guesser::new(candidates_buf.take().unwrap());
    let mut attempts = ArrayVec::<Word, 6>::new();
    for turn in 1..=6 {
      let guess = guesser.guess().unwrap();
      attempts.push(*guess);
      let stats = check_word(*word, *guess);
      if guess == word {
        games.push((true, *word, attempts));
        candidates_buf = Some(guesser.extract_resources());
        continue 'rounds;
      }
      guesser.analyze(std::array::from_fn(|i| (guess[i], stats[i])));
      guesser.prune(turn);
    }
    games.push((false, *word, attempts));
    candidates_buf = Some(guesser.extract_resources());
  }

  // send statistics to TSV
  {
    if let Ok(file) = std::fs::File::create("stats.tsv") {
      use std::io::Write;
      const FALSE: Word = Word::from_bytes(*b"FALSE").unwrap();
      let mut buf_writer = std::io::BufWriter::new(file);
      _ = write!(buf_writer, "\"Word\"\t\"Success\"\t\"Turns\"\t\"Turn 1 word\"\t\"Turn 2 word\"\t\"Turn 3 word\"\t\"Turn 4 word\"\t\"Turn 5 word\"\t\"Turn 6 word\"");
      for (success, word, attempts) in games.iter() {
        if *success {
          _ = write!(buf_writer, "\n\"{}{word}\"\tTRUE\t{}", if word == &FALSE { "'" } else { "" }, attempts.len());
        } else {
          _ = write!(buf_writer, "\n\"{}{word}\"\tFALSE\t#N/A", if word == &FALSE { "'" } else { "" });
        }
        for attempt in attempts {
          _ = write!(buf_writer, "\t\"{}{attempt}\"", if attempt == &FALSE { "'" } else { "" });
        }
      }
      _ = buf_writer.flush();
    }
  }

  let turns: Vec<_> = games.iter()
    .map(|(success, _, words)| success.then(|| words.len() as u32))
    .collect();

  let mut successes: Vec<_> = turns.iter()
    .copied()
    .filter_map(|n| n)
    .collect();

  successes.sort();

  let won = successes.len();
  let lost = turns.len() - won;
  let win_probability = won as f64 / turns.len() as f64;
  println!("\
    games won: {won}\n\
    games lost: {lost}\n\
    win probability: {win_probability}\
  ");

  if !successes.is_empty() {
    let min = successes.first().copied().unwrap();
    let max = successes.last().copied().unwrap();
    let range = max - min;
    let mean = successes.iter().copied().map(|x| x as f64).sum::<f64>() / successes.len() as f64;
    let q1 = successes[1*successes.len() / 4];
    let q2 = successes[2*successes.len() / 4];
    let q3 = successes[3*successes.len() / 4];
    let iqr = q3 - q1;

    println!("\
      min turns: {min}\n\
      max turns: {max}\n\
      range: {range}\n\
      mean: {mean}\n\
      Q1: {q1}\n\
      median: {q2}\n\
      Q3: {q3}\n\
      IQR: {iqr}\
    ");

    let mut slice = &successes[..];
    const COLORS: [&str; 7] = ["🟪", "🟦", "🟩", "🟨", "🟧", "🟥", "\u{2B1C}"];
    const COLOR_BAR: &str = "🟥🟥🟥🟥🟥🟥🟧🟧🟧🟧🟧🟧🟧🟨🟨🟨🟨🟨🟨🟨🟨🟩🟩🟩🟩🟩🟩🟩🟩🟦🟦🟦🟦🟦🟦🟦🟪🟪🟪🟪🟪🟪";
    const SCALE: usize = COLOR_BAR.len()/'🟥'.len_utf8();
    const HEADERS: [&str; 3] = [
      "\nwins per turn:\n",
      "\nprobability of winning on a turn:\n",
      "\nprobability of winning on a turn, given that turn has been reached:\n",
      // "\nprobability of having won in n turns or fewer:\n",
      // "\nprobability of needing at least n turns to win:\n",
    ];
    let mut output = String::with_capacity(
      HEADERS.iter()
        .map(|s| s.len())
        .sum::<usize>() +
      ("_: 00000 \n".len() + COLOR_BAR.len())*(6*HEADERS.len() + 1)
    );

    let mut ranges = [0; 7];
    for turn in 0..6 {
      let n = slice.partition_point(|&t| t == turn + 1);
      ranges[turn as usize] = n;
      slice = &slice[n..];
    }
    ranges[6] = lost;
    let most = ranges.iter().copied().max().unwrap();

    use std::fmt::Write;

    output.push_str(HEADERS[0]);
    for (turn, n) in ranges.iter().copied().enumerate() {
      write!(&mut output, "{}: {n:>5} {:⬛<SCALE$}\n",
        if turn == 6 { 'L' } else { char::from(b'1' + turn as u8) },
        COLORS[turn as usize].repeat((SCALE as f64*n as f64/most as f64).round() as usize),
      ).unwrap();
    }
    output.push_str(HEADERS[1]);
    for (turn, n) in ranges.iter().take(6).copied().enumerate() {
      let p = n as f64/turns.len() as f64;
      write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
        turn + 1,
        &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
      ).unwrap();
    }
    output.push_str(HEADERS[2]);
    let mut contestants = turns.len();
    for (turn, n) in ranges.iter().take(6).copied().enumerate() {
      if contestants == 0 {
        write!(&mut output, "{}: no data, always won before this turn", turn + 1).unwrap();
      } else {
        let p = n as f64/contestants as f64;
        write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
          turn + 1,
          &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
        ).unwrap();
      }
      contestants -= n;
    }
    // output.push_str(HEADERS[3]);
    // let mut p = 0.0;
    // for (turn, n) in ranges.iter().take(6).copied().enumerate() {
    //   p += n as f64/turns.len() as f64;
    //   write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
    //     turn + 1,
    //     &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
    //   ).unwrap();
    // }
    // output.push_str(HEADERS[4]);
    // let mut p = 1.0;
    // for (turn, n) in ranges.iter().take(6).copied().enumerate() {
    //   p -= n as f64/turns.len() as f64;
    //   write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
    //     turn + 1,
    //     &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
    //   ).unwrap();
    // }
    print!("{output}");
  }
}

#[cfg(test)]
mod test {
  use crate::{dictionary::FIVE_LETTER_WORDS, guess::Guesser, play::check_word, Attempts};
  use rand::{prelude::*, rng};

  #[test]
  fn test_random() {
    let mut rng = rng();
    let mut candidates_buf = Some(Vec::new());
    let mut final_boards = Vec::new();
    'rounds: for (round, word) in FIVE_LETTER_WORDS.choose_multiple(&mut rng, 10).enumerate() {
      println!("\nround {round}:");
      let mut guesser = Guesser::new(candidates_buf.take().expect("should always have buffer at round start"));
      let mut guesses = Vec::new();
      let mut attempts = Attempts::new();
      for turn in 1..=6 {
        let guess = guesser.guess().expect("should always have a suggestion");
        guesses.push((*guess, guesser.candidates().len()));
        let stats = check_word(*word, *guess);
        attempts.push(stats);
        if guess == word {
          println!("won on turn {turn}");
          final_boards.push((round, word, attempts, guesses));
          candidates_buf = Some(guesser.extract_resources());
          continue 'rounds;
        }
        guesser.analyze(std::array::from_fn(|i| (guess[i], stats[i])));
        guesser.prune(turn);
        assert!(guesser.candidates().contains(word), "should never remove actual word from candidates");
      }
      println!("failed to identify word in alloted time :(");
      final_boards.push((round, word, attempts, guesses));
      candidates_buf = Some(guesser.extract_resources());
    }
    for (round, word, board, guesses) in final_boards.into_iter() {
      println!("round {round}: {word}\n{board}");
      for (guess, candidate_count) in guesses {
        println!("{guess} [{candidate_count} candidates]");
      }
      println!();
    }
  }
}
