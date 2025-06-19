use std::{io::stdin, sync::LazyLock};
use arrayvec::ArrayVec;
use guess::*;
use crate::{dictionary::FIVE_LETTER_WORDS, play::Player, word::{Letter, Word}};

pub static VERBOSE_MESSAGES: LazyLock<bool> = LazyLock::new(||
  std::env::args().any(|s| matches!(s.as_str(), "-v" | "--verbose"))
);

static IS_STATS_RUN: LazyLock<bool> = LazyLock::new(||
  std::env::args().any(|s| matches!(s.as_str(), "-s" | "--stats"))
);

mod word;
mod dictionary;
mod guess;
mod play;

pub struct Attempts(ArrayVec::<[CharStatus; 5], 6>);

impl Attempts {
  pub const fn new() -> Self {
    Self(ArrayVec::new_const())
  }

  pub fn push(&mut self, stats: [CharStatus; 5]) {
    self.0.push(stats);
  }
}

impl std::fmt::Display for Attempts {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for row in 0..self.0.len() {
      for col in self.0[row] {
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
  if *IS_STATS_RUN {
    statistics();
  } else {
    let mut rng = rand::rng();
    let mut buf = String::with_capacity(12);
    let mut guesser = Guesser::new(Vec::new());
    let mut attempts = Attempts::new();

    for turn in 1..=6 {
      println!("turn {turn} ({} remaining):", 6 - turn);
      if let Some(s) = guesser.guess(turn, &mut rng) {
        println!("suggestion: {s}");
      } else {
        println!("no such word exists in my dictionary");
        return;
      }
      buf.clear();
      stdin().read_line(&mut buf).unwrap();
      buf.truncate(buf.trim_end().len());
      if buf.trim_end() == "exit" { return; }
      stdin().read_line(&mut buf).unwrap();
      buf.truncate(buf.trim_end().len());
      assert!(buf.len() == 10);
      let bytes = buf.as_bytes();
      let feedback = std::array::from_fn(|i| (
        Letter::from_u8(bytes[i].to_ascii_uppercase())
          .expect("unknown format"),
        match bytes[i + 5] {
          b'+' => CharStatus::Confirmed,
          b'?' => CharStatus::Required,
          b'_' => CharStatus::Excluded,
          _ => panic!("unknown format"),
        },
      ));
      attempts.push(feedback.map(|(_, stat)| stat));
      if attempts.0.last() == Some(&[CharStatus::Confirmed; 5]) {
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

pub fn statistics() {
  let mut rng = rand::rng();
  let mut candidates_buf = Some(Vec::new());
  let mut turns = Vec::with_capacity(FIVE_LETTER_WORDS.len());
  'rounds: for word in FIVE_LETTER_WORDS.iter() {
    let game = Player::new(*word);
    let mut guesser = Guesser::new(candidates_buf.take().unwrap());
    for turn in 1..=6 {
      let guess = guesser.guess(turn, &mut rng).unwrap();
      let stats = game.check(guess);
      if guess == word {
        turns.push(Some(turn));
        candidates_buf = Some(guesser.extract_resources());
        continue 'rounds;
      }
      guesser.analyze(std::array::from_fn(|i| (guess[i], stats[i])));
      guesser.prune(turn);
    }
    turns.push(None);
    candidates_buf = Some(guesser.extract_resources());
  }

  let mut successes: Vec<_> = turns.iter()
    .copied()
    .filter_map(|x| x)
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
    const COLORS: [&str; 7] = ["🟪", "🟦", "🟩", "🟨", "🟧", "🟥", "⬜"];
    const COLOR_BAR: &str = "🟥🟥🟥🟥🟥🟥🟧🟧🟧🟧🟧🟧🟧🟨🟨🟨🟨🟨🟨🟨🟨🟩🟩🟩🟩🟩🟩🟩🟩🟦🟦🟦🟦🟦🟦🟦🟪🟪🟪🟪🟪🟪";
    const SCALE: usize = COLOR_BAR.len()/'🟥'.len_utf8();
    const HEADERS: [&str; 5] = [
      "\nwins per turn:\n",
      "\nprobability of winning on a turn:\n",
      "\nprobability of winning on a turn, given that turn has been reached:\n",
      "\nprobability of having won in n turns or fewer:\n",
      "\nprobability of needing at least n turns to win:\n",
    ];
    let mut output = String::with_capacity(
      HEADERS.iter()
        .map(|s| s.len())
        .sum::<usize>() +
      ("_: 00000 \n".len() + COLOR_BAR.len())*(6*4 + 1)
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
      let p = n as f64/contestants as f64;
      write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
        turn + 1,
        &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
      ).unwrap();
      contestants -= n;
    }
    output.push_str(HEADERS[3]);
    let mut p = 0.0;
    for (turn, n) in ranges.iter().take(6).copied().enumerate() {
      p += n as f64/turns.len() as f64;
      write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
        turn + 1,
        &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
      ).unwrap();
    }
    output.push_str(HEADERS[4]);
    let mut p = 1.0;
    for (turn, n) in ranges.iter().take(6).copied().enumerate() {
      p -= n as f64/turns.len() as f64;
      write!(&mut output, "{}: {p:>1.3} {:⬛<SCALE$}\n",
        turn + 1,
        &COLOR_BAR[..'🟥'.len_utf8()*(SCALE as f64*p).round() as usize],
      ).unwrap();
    }
    print!("{output}");
  }
}

#[cfg(test)]
mod test {
  use crate::{dictionary::FIVE_LETTER_WORDS, guess::Guesser, play::Player, Attempts};
  use rand::{prelude::*, rng};

  #[test]
  fn test_random() {
    let mut rng = rng();
    let mut candidates_buf = Some(Vec::new());
    let mut final_boards = Vec::new();
    'rounds: for (round, word) in FIVE_LETTER_WORDS.choose_multiple(&mut rng, 10).enumerate() {
      println!("\nround {round}:");
      let game = Player::new(*word);
      let mut guesser = Guesser::new(candidates_buf.take().expect("should always have buffer at round start"));
      let mut guesses = Vec::new();
      let mut attempts = Attempts::new();
      for turn in 1..=6 {
        let guess = guesser.guess(turn, &mut rng).expect("should always have a suggestion");
        guesses.push((*guess, guesser.candidates().len()));
        let stats = game.check(guess);
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

  #[test]
  fn test_statistics() {
    super::statistics()
  }
}
