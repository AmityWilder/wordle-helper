#![allow(unused)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Letter {
  A = b'A',
  B = b'B',
  C = b'C',
  D = b'D',
  E = b'E',
  F = b'F',
  G = b'G',
  H = b'H',
  I = b'I',
  J = b'J',
  K = b'K',
  L = b'L',
  M = b'M',
  N = b'N',
  O = b'O',
  P = b'P',
  Q = b'Q',
  R = b'R',
  S = b'S',
  T = b'T',
  U = b'U',
  V = b'V',
  W = b'W',
  X = b'X',
  Y = b'Y',
  Z = b'Z',
}

impl std::fmt::Display for Letter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    char::from(*self as u8).fmt(f)
  }
}

impl Letter {
  pub const fn from_u8(b: u8) -> Option<Self> {
    if matches!(b, b'A'..=b'Z') {
      Some(unsafe { Self::from_u8_unchecked(b) })
    } else {
      None
    }
  }

  pub const unsafe fn from_u8_unchecked(b: u8) -> Self {
    unsafe { std::mem::transmute(b) }
  }

  pub const fn to_u8(self) -> u8 {
    self as u8
  }

  /// - A -> 0
  /// - B -> 1
  /// - C -> 2
  /// - ...
  /// - Z -> 25
  pub const fn index(self) -> usize {
    (self as u8 - b'A') as usize
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Word(pub [Letter; 5]);

impl std::ops::Deref for Word {
  type Target = [Letter; 5];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl std::ops::DerefMut for Word {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl Word {
  pub const fn from_bytes(bytes: [u8; 5]) -> Option<Self> {
    if matches!(bytes, [b'A'..=b'Z', b'A'..=b'Z', b'A'..=b'Z', b'A'..=b'Z', b'A'..=b'Z']) {
      Some(unsafe { Self::from_bytes_unchecked(bytes) })
    } else {
      None
    }
  }

  pub const unsafe fn from_bytes_unchecked(bytes: [u8; 5]) -> Self {
    unsafe { std::mem::transmute(bytes) }
  }

  pub const fn to_bytes(self) -> [u8; 5] {
    let [c0, c1, c2, c3, c4] = self.0;
    [c0 as u8, c1 as u8, c2 as u8, c3 as u8, c4 as u8]
  }

  pub const fn as_bytes(&self) -> &[u8; 5] {
    unsafe { std::mem::transmute(&self.0) }
  }

  pub const fn as_str(&self) -> &str {
    unsafe { str::from_utf8_unchecked(self.as_bytes()) }
  }
}

impl std::fmt::Display for Word {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    self.as_str().fmt(f)
  }
}
