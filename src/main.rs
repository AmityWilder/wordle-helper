use std::io::stdin;
use arrayvec::ArrayVec;
use guess::*;

pub const VERBOSE_MESSAGES: bool = false;

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
  let mut buf = String::with_capacity(12);
  let mut guesser = Guesser::new(Vec::new());
  let mut attempts = Attempts::new();

  for turn in 1..=6 {
    println!("turn {turn} ({} remaining):", 6 - turn);
    if let Some(s) = guesser.guess() {
      println!("suggestion: {}", unsafe { str::from_utf8_unchecked(s) });
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
      bytes[i].to_ascii_uppercase(),
      match bytes[i + 5] {
        b'+' => CharStatus::Confirmed,
        b'?' => CharStatus::Required,
        b'_' => CharStatus::Excluded,
        _ => panic!("unknown format"),
      },
    ));
    attempts.push(feedback.map(|(_, stat)| stat));
    guesser.analyze(feedback);
    if let Some(word) = guesser.confirmed_word() {
      println!("{attempts}");
      println!("success! winning word: {}", unsafe { str::from_utf8_unchecked(&word) });
      return;
    }
    guesser.prune();
    print!("candidates:");
    for (n, word) in (0..7).cycle().zip(guesser.candidates()) {
      if n == 0 { println!(); }
      print!("{} ", unsafe { str::from_utf8_unchecked(word) });
    }
    println!();
    println!("{attempts}");
  }
  println!("game over");
}

#[cfg(test)]
mod test {
  use crate::{dictionary::FIVE_LETTER_WORDS, guess::Guesser, play::Player, Attempts};
  use rand::{prelude::*, rng};

  #[test]
  fn test() {
    let mut candidates_buf = Some(Vec::new());
    let mut rng = rng();
    let mut final_boards = Vec::new();
    'rounds: for (round, word) in FIVE_LETTER_WORDS.choose_multiple(&mut rng, 10).enumerate() {
      println!("\nround {round}:");
      let game = Player::new(*word);
      let mut guesser = Guesser::new(candidates_buf.take().expect("should always have buffer at round start"));
      let mut guesses = Vec::new();
      let mut attempts = Attempts::new();
      for turn in 1..=6 {
        let guess = guesser.guess().expect("should always have a suggestion");
        guesses.push((*guess, guesser.candidates().len()));
        let stats = game.check(guess);
        attempts.push(stats);
        guesser.analyze(std::array::from_fn(|i| (guess[i], stats[i])));
        if let Some(winner) = guesser.confirmed_word() {
          assert_eq!(
            unsafe { str::from_utf8_unchecked(&winner) },
            unsafe { str::from_utf8_unchecked(word) },
          );
          println!("won on turn {turn}");
          final_boards.push((round, word, attempts, guesses));
          candidates_buf = Some(guesser.extract_resources());
          continue 'rounds;
        }
        guesser.prune();
        assert!(guesser.candidates().contains(word), "should never remove actual word from candidates");
      }
      println!("failed to identify word in alloted time :(");
      final_boards.push((round, word, attempts, guesses));
      candidates_buf = Some(guesser.extract_resources());
    }
    for (round, word, board, guesses) in final_boards.into_iter() {
      println!("round {round}: {}\n{board}", unsafe { str::from_utf8_unchecked(word) });
      for (guess, candidate_count) in guesses {
        println!("{} [{candidate_count} candidates]", unsafe { str::from_utf8_unchecked(&guess) });
      }
      println!();
    }
  }

  #[test]
  fn test_statistics() {
    let mut candidates_buf = Some(Vec::new());
    let mut turns = Vec::with_capacity(FIVE_LETTER_WORDS.len());
    'rounds: for word in FIVE_LETTER_WORDS.iter() {
      let game = Player::new(*word);
      let mut guesser = Guesser::new(candidates_buf.take().unwrap());
      for i in 0u32..6 {
        let guess = guesser.guess().unwrap();
        let stats = game.check(guess);
        guesser.analyze(std::array::from_fn(|i| (guess[i], stats[i])));
        if guesser.confirmed_word().is_some() {
          turns.push(Some(i));
          candidates_buf = Some(guesser.extract_resources());
          continue 'rounds;
        }
        guesser.prune();
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
      let color = ['🟪', '🟦', '🟩', '🟨', '🟧', '🟥', '⬜'];
      let mut ranges = [0; 7];
      for turns in 0..6 {
        let n = slice.partition_point(|&t| t == turns);
        ranges[turns as usize] = n;
        slice = &slice[n..];
      }
      ranges[6] = lost;
      let most = ranges.iter().copied().max().unwrap();
      for (turns, n) in ranges.iter().copied().enumerate() {
        if turns == 6 {
          print!("_");
        } else {
          print!("{turns}");
        }
        print!(": {n:>5} ");
        let col = color[turns as usize];
        for _ in 0..((n as f64/most as f64)*20.0).floor() as usize {
          print!("{col}");
        }
        println!();
      }
    }
  }
}
