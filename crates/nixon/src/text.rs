use core::fmt;

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

impl fmt::Debug for TextSize {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.0.fmt(formatter)
  }
}

/// A half-open byte range in a Nix source file.
#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
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
}

impl fmt::Debug for TextRange {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}..{}", self.start.get(), self.end.get())
  }
}
