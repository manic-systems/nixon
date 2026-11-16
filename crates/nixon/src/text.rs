use core::{
  fmt,
  ops::{
    Add,
    Sub,
  },
};

/// A byte offset into a Nix source file.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextSize(u32);

impl TextSize {
  /// The largest source offset Nixon can represent.
  pub const MAX: Self = Self(u32::MAX);

  /// Creates an offset from its compact representation.
  #[must_use]
  pub const fn new(value: u32) -> Self {
    Self(value)
  }

  /// Returns the offset as a `u32`.
  #[must_use]
  pub const fn get(self) -> u32 {
    self.0
  }

  /// Returns the offset as a platform-sized integer.
  #[must_use]
  pub const fn as_usize(self) -> usize {
    self.0 as usize
  }
}

impl From<u32> for TextSize {
  fn from(value: u32) -> Self {
    Self(value)
  }
}

impl From<TextSize> for u32 {
  fn from(value: TextSize) -> Self {
    value.0
  }
}

impl From<TextSize> for usize {
  fn from(value: TextSize) -> Self {
    value.as_usize()
  }
}

impl Add for TextSize {
  type Output = Self;

  fn add(self, rhs: Self) -> Self {
    Self(self.0 + rhs.0)
  }
}

impl Sub for TextSize {
  type Output = Self;

  fn sub(self, rhs: Self) -> Self {
    Self(self.0 - rhs.0)
  }
}

impl fmt::Debug for TextSize {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.0.fmt(formatter)
  }
}

/// A half-open byte range in a Nix source file.
///
/// Ranges order by start offset, then by end offset.
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TextRange {
  start: TextSize,
  end:   TextSize,
}

impl TextRange {
  /// Creates a range from two byte offsets.
  #[must_use]
  pub const fn new(start: TextSize, end: TextSize) -> Self {
    Self { start, end }
  }

  /// Creates a range from a start offset and byte length.
  #[must_use]
  pub const fn at(start: TextSize, len: TextSize) -> Self {
    Self::new(start, TextSize::new(start.get() + len.get()))
  }

  /// Returns the first byte offset in the range.
  #[must_use]
  pub const fn start(self) -> TextSize {
    self.start
  }

  /// Returns the byte offset immediately after the range.
  #[must_use]
  pub const fn end(self) -> TextSize {
    self.end
  }

  /// Returns the number of bytes covered by the range.
  #[must_use]
  pub const fn len(self) -> TextSize {
    TextSize::new(self.end.get() - self.start.get())
  }

  /// Returns whether the range contains no bytes.
  #[must_use]
  pub const fn is_empty(self) -> bool {
    self.start.get() == self.end.get()
  }

  /// Returns whether `offset` lies in the range, excluding its end.
  #[must_use]
  pub const fn contains(self, offset: TextSize) -> bool {
    self.start.get() <= offset.get() && offset.get() < self.end.get()
  }

  /// Returns whether `offset` lies in the range, including its end.
  #[must_use]
  pub const fn contains_inclusive(self, offset: TextSize) -> bool {
    self.start.get() <= offset.get() && offset.get() <= self.end.get()
  }

  /// Returns whether `other` lies entirely within the range.
  #[must_use]
  pub const fn contains_range(self, other: Self) -> bool {
    self.start.get() <= other.start.get() && other.end.get() <= self.end.get()
  }

  /// Returns the overlap of two ranges, or `None` if they are disjoint.
  ///
  /// Ranges that only touch intersect in an empty range.
  #[must_use]
  pub const fn intersect(self, other: Self) -> Option<Self> {
    let start = if self.start.get() > other.start.get() {
      self.start
    } else {
      other.start
    };
    let end = if self.end.get() < other.end.get() {
      self.end
    } else {
      other.end
    };
    if start.get() <= end.get() {
      Some(Self::new(start, end))
    } else {
      None
    }
  }

  /// Returns the smallest range covering both ranges.
  #[must_use]
  pub const fn cover(self, other: Self) -> Self {
    let start = if self.start.get() < other.start.get() {
      self.start
    } else {
      other.start
    };
    let end = if self.end.get() > other.end.get() {
      self.end
    } else {
      other.end
    };
    Self::new(start, end)
  }
}

impl fmt::Debug for TextRange {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}..{}", self.start.get(), self.end.get())
  }
}
