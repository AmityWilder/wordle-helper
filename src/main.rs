mod dictionary;
use std::io::stdin;
use arrayvec::ArrayVec;
use bitflags::bitflags;

bitflags!{
  #[derive(Debug, Clone, Copy)]
  pub struct Positions: u8 {
    const P1 = 1 << 0;
    const P2 = 1 << 1;
    const P3 = 1 << 2;
    const P4 = 1 << 3;
    const P5 = 1 << 4;
  }
}

impl Positions {
  pub const fn from_index(index: usize) -> Option<Self> {
    Self::from_bits(1u8 << index)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
  Excluded,
  Required,
  Confirmed,
}

fn main() {
  let mut candidates = Vec::new();
  let mut excluded = ArrayVec::<u8, {26 - 5}>::new();
  let mut required = ArrayVec::<(u8, Positions), 5>::new();
  let mut confirmed = [None; 5];
  let mut buf = String::with_capacity(12);

  for turn in 1..=6 {
    println!("turn {turn}:");
    buf.clear();
    stdin().read_line(&mut buf).unwrap();
    buf.truncate(buf.trim_end().len());
    if buf.trim_end() == "exit" { break; }
    stdin().read_line(&mut buf).unwrap();
    buf.truncate(buf.trim_end().len());
    assert!(buf.len() == 10);
    let bytes = buf.as_bytes();
    for i in 0..5 {
      let ch = bytes[i].to_ascii_uppercase();
      let stat = match bytes[i + 5] {
        b'+' => Status::Confirmed,
        b'?' => Status::Required,
        b'_' => Status::Excluded,
        _ => panic!("unknown format"),
      };
      match stat {
        Status::Excluded => if let Err(pos) = excluded.binary_search(&ch) {
          excluded.insert(pos, ch);
        }
        Status::Required => {
          let pos = Positions::from_index(i).unwrap();
          match required.binary_search_by_key(&ch, |(r, _)| *r) {
            Ok(idx) => required[idx].1.insert(pos),
            Err(idx) => required.insert(idx, (ch, pos)),
          }
        }
        Status::Confirmed => confirmed[i] = Some(ch),
      }
    }
    let include = |word: &[u8; 5]| -> bool {
      // Must contain all confirmed
      word.iter().copied().zip(confirmed.iter().copied())
        .all(|(a, b)| b.is_none_or(|b| a == b))
      &&
      // Must contain none excluded
      !word.iter().any(|ch| excluded.binary_search(ch).is_ok())
      &&
      // Must contain all required
      required.iter().copied().all(|(r, p)| {
        word.contains(&r) &&
        word.iter().copied()
          .enumerate()
          // but only in an open space
          .filter(|&(i, ch)| confirmed[i].is_none() && ch == r)
          // where that character has not been tried yet
          .all(|(i, _)| !p.contains(Positions::from_index(i).unwrap()))
      })
    };
    if candidates.is_empty() {
      candidates.extend(
        dictionary::FIVE_LETTER_WORDS
          .iter()
          .copied()
          .filter(include)
      );
    } else {
      candidates
        .retain(include);
    }
    print!("candidates:");
    for (n, word) in (0..7).cycle().zip(candidates.iter()) {
      if n == 0 { println!(); }
      print!("{} ", unsafe { str::from_utf8_unchecked(word) });
    }
    println!();
  }
}
