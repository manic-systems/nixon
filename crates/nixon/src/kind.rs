/// The lexical and grammatical kinds in a Nixon syntax tree.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum SyntaxKind {
  /// Invalid or otherwise unrecognized input.
  Error,
  /// The end of the source.
  Eof,
  /// Spaces, tabs, or line endings.
  Whitespace,
  /// A `#` line comment.
  LineComment,
  /// A `/* ... */` block comment.
  BlockComment,
  /// A `/** ... */` documentation comment.
  DocComment,
  /// An identifier.
  Identifier,
  /// An integer literal.
  Integer,
  /// A floating-point literal.
  Float,
  /// Raw text inside a string or interpolated path.
  StringFragment,
  /// Raw text inside an interpolated path.
  PathFragment,
  /// A path literal without interpolation.
  Path,
  /// A search-path literal such as `<nixpkgs>`.
  SearchPath,
  /// A URI literal.
  Uri,
  /// `if`.
  IfKeyword,
  /// `then`.
  ThenKeyword,
  /// `else`.
  ElseKeyword,
  /// `assert`.
  AssertKeyword,
  /// `with`.
  WithKeyword,
  /// `let`.
  LetKeyword,
  /// `in`.
  InKeyword,
  /// `rec`.
  RecKeyword,
  /// `inherit`.
  InheritKeyword,
  /// `or`.
  OrKeyword,
  /// `(`.
  LeftParen,
  /// `)`.
  RightParen,
  /// `{`.
  LeftBrace,
  /// `}`.
  RightBrace,
  /// `[`.
  LeftBracket,
  /// `]`.
  RightBracket,
  /// `"`.
  Quote,
  /// `''` at an indented-string boundary.
  IndentedQuote,
  /// `${`.
  InterpolationStart,
  /// `,`.
  Comma,
  /// `;`.
  Semicolon,
  /// `:`.
  Colon,
  /// `@`.
  At,
  /// `.`.
  Dot,
  /// `...`.
  Ellipsis,
  /// `=`.
  Assign,
  /// `==`.
  Equal,
  /// `!=`.
  NotEqual,
  /// `<`.
  Less,
  /// `>`.
  Greater,
  /// `<=`.
  LessEqual,
  /// `>=`.
  GreaterEqual,
  /// `+`.
  Plus,
  /// `-`.
  Minus,
  /// `*`.
  Star,
  /// `/`.
  Slash,
  /// `!`.
  Bang,
  /// `&&`.
  And,
  /// `||`.
  Or,
  /// `->`.
  Implication,
  /// `//`.
  Update,
  /// `++`.
  Concatenate,
  /// `?`.
  Question,
  /// `<|`.
  PipeFrom,
  /// `|>`.
  PipeInto,
  /// A complete source file.
  Root,
  /// A syntactically invalid region.
  ErrorNode,
  /// A literal expression.
  Literal,
  /// A quoted or indented string expression.
  String,
  /// A string interpolation.
  Interpolation,
  /// An interpolated path expression.
  PathExpression,
  /// A parenthesized expression.
  Parenthesized,
  /// A list expression.
  List,
  /// An attribute set expression.
  AttributeSet,
  /// A binding.
  Binding,
  /// An attribute path.
  AttributePath,
  /// An `inherit` binding.
  Inherit,
  /// A `let ... in ...` expression.
  LetIn,
  /// A legacy `let { ... }` expression.
  LegacyLet,
  /// A conditional expression.
  IfThenElse,
  /// An assertion expression.
  Assert,
  /// A `with` expression.
  With,
  /// A function expression.
  Lambda,
  /// A destructured function parameter set.
  FormalSet,
  /// A formal function parameter.
  Formal,
  /// A function application.
  Apply,
  /// An attribute selection.
  Select,
  /// An attribute-existence test.
  HasAttribute,
  /// A unary operation.
  UnaryOperation,
  /// A binary operation.
  BinaryOperation,
}

impl SyntaxKind {
  /// Returns whether the kind is source trivia.
  #[must_use]
  pub const fn is_trivia(self) -> bool {
    matches!(
      self,
      Self::Whitespace
        | Self::LineComment
        | Self::BlockComment
        | Self::DocComment
    )
  }

  /// Returns whether the kind represents a token rather than a tree node.
  #[must_use]
  pub const fn is_token(self) -> bool {
    (self as u8) < (Self::Root as u8)
  }
}
