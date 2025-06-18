mod dictionary;
mod guess;
use std::io::stdin;
use arrayvec::ArrayVec;
use guess::*;

fn main() {
  let mut rng = rand::rng();
  let mut buf = String::with_capacity(12);
  let mut guesser = Guesser::new();
  let mut attempts = ArrayVec::<_, 6>::new();

  for turn in 1..=6 {
    println!("turn {turn} ({} remaining):", 6 - turn);
    if let Some(s) = guesser.suggestion(&mut rng) {
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
    for row in &attempts {
      for col in row {
        print!("{col}");
      }
      println!();
    }
    guesser.submit_feedback(feedback);
    if let Some(word) = guesser.confirmed_word() {
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
  }
}
