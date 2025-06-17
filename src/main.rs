mod dictionary;
use std::io::stdin;

pub fn filter_words<'a>(
  dictionary: &'a [[u8; 5]],
  confirmed: &[Option<u8>; 5],
  required: &[u8],
  confirmed_absent: &[u8],
) -> impl Iterator<Item = &'a [u8; 5]> {
  dictionary
    .iter()
    .filter(|candidate|
      required.iter().copied()
        .all(|r| candidate.contains(&r)) &&
      !candidate.iter().copied()
        .zip(confirmed.iter().copied())
        .any(|(c, confirmed)|
          confirmed_absent.contains(&c) ||
          confirmed.is_some_and(|x| c != x)
        )
    )
}

fn main() {
  let mut confirmed = [None; 5];
  let mut required = Vec::with_capacity(5);
  let mut confirmed_absent = Vec::with_capacity(26 - 5);

  let mut buf = String::with_capacity(12);

  loop {
    buf.clear();
    stdin().read_line(&mut buf).unwrap();
    buf.truncate(buf.trim_end().len());
    if buf.trim_end() == "exit" { break; }
    stdin().read_line(&mut buf).unwrap();
    buf.truncate(buf.trim_end().len());
    assert!(buf.len() == 10);
    for i in 0..5 {
      let ch = buf.as_bytes()[i].to_ascii_uppercase();
      let stat = buf.as_bytes()[i + 5];
      match stat {
        b'@' => confirmed[i] = Some(ch),
        b'?' => if !required.contains(&ch) { required.push(ch) }
        b'_' => if !confirmed_absent.contains(&ch) { confirmed_absent.push(ch) }
        _ => panic!("unknown format"),
      }
    }
    println!("Confirmed: {confirmed:?}");
    println!("Required: {required:?}");
    println!("Confirmed Absent: {confirmed_absent:?}");
    println!("Candidates:");
    for word in filter_words(&dictionary::FIVE_LETTER_WORDS, &confirmed, &required, &confirmed_absent) {
      println!("- {}", unsafe { str::from_utf8_unchecked(word) });
    }
  }
}
