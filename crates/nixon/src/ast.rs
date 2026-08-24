//! Typed views over Nixon's lossless syntax tree.

use crate::{
  Element,
  Node,
  SyntaxKind,
  TokenNode,
  tree::Children,
};

/// A typed syntax node.
pub trait AstNode<'doc, 'src>: Copy {
  /// Attempts to cast an untyped node.
  fn cast(node: Node<'doc, 'src>) -> Option<Self>;

  /// Returns the underlying syntax node.
  fn syntax(self) -> Node<'doc, 'src>;
}

macro_rules! ast_node {
  ($name:ident, $kind:ident) => {
    #[doc = concat!("A typed view of a `", stringify!($kind), "` node.")]
    #[derive(Clone, Copy, Debug)]
    pub struct $name<'doc, 'src>(Node<'doc, 'src>);

    impl<'doc, 'src> AstNode<'doc, 'src> for $name<'doc, 'src> {
      fn cast(node: Node<'doc, 'src>) -> Option<Self> {
        (node.kind() == SyntaxKind::$kind).then_some(Self(node))
      }

      fn syntax(self) -> Node<'doc, 'src> {
        self.0
      }
    }
  };
}

ast_node!(Root, Root);
ast_node!(Literal, Literal);
ast_node!(StringExpression, String);
ast_node!(Interpolation, Interpolation);
ast_node!(PathExpression, PathExpression);
ast_node!(Parenthesized, Parenthesized);
ast_node!(List, List);
ast_node!(AttributeSet, AttributeSet);
ast_node!(Binding, Binding);
ast_node!(AttributePath, AttributePath);
ast_node!(Inherit, Inherit);
ast_node!(LetIn, LetIn);
ast_node!(LegacyLet, LegacyLet);
ast_node!(IfThenElse, IfThenElse);
ast_node!(Assert, Assert);
ast_node!(With, With);
ast_node!(Lambda, Lambda);
ast_node!(FormalSet, FormalSet);
ast_node!(Formal, Formal);
ast_node!(Apply, Apply);
ast_node!(Select, Select);
ast_node!(HasAttribute, HasAttribute);
ast_node!(UnaryOperation, UnaryOperation);
ast_node!(BinaryOperation, BinaryOperation);
ast_node!(ErrorNode, ErrorNode);

/// Any expression node in a Nixon syntax tree.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Expression<'doc, 'src> {
  /// An identifier, number, plain path, search path, or URI.
  Literal(Literal<'doc, 'src>),
  /// A quoted or indented string.
  String(StringExpression<'doc, 'src>),
  /// An interpolated path.
  Path(PathExpression<'doc, 'src>),
  /// A parenthesized expression.
  Parenthesized(Parenthesized<'doc, 'src>),
  /// A list.
  List(List<'doc, 'src>),
  /// An attribute set.
  AttributeSet(AttributeSet<'doc, 'src>),
  /// A `let ... in ...` expression.
  LetIn(LetIn<'doc, 'src>),
  /// A legacy `let { ... }` expression.
  LegacyLet(LegacyLet<'doc, 'src>),
  /// A conditional expression.
  IfThenElse(IfThenElse<'doc, 'src>),
  /// An assertion expression.
  Assert(Assert<'doc, 'src>),
  /// A `with` expression.
  With(With<'doc, 'src>),
  /// A function expression.
  Lambda(Lambda<'doc, 'src>),
  /// A function application.
  Apply(Apply<'doc, 'src>),
  /// An attribute selection.
  Select(Select<'doc, 'src>),
  /// An attribute-existence test.
  HasAttribute(HasAttribute<'doc, 'src>),
  /// A unary operation.
  Unary(UnaryOperation<'doc, 'src>),
  /// A binary operation.
  Binary(BinaryOperation<'doc, 'src>),
  /// A recovered malformed expression.
  Error(ErrorNode<'doc, 'src>),
}

impl<'doc, 'src> AstNode<'doc, 'src> for Expression<'doc, 'src> {
  fn cast(node: Node<'doc, 'src>) -> Option<Self> {
    Some(match node.kind() {
      SyntaxKind::Literal => Self::Literal(Literal(node)),
      SyntaxKind::String => Self::String(StringExpression(node)),
      SyntaxKind::PathExpression => Self::Path(PathExpression(node)),
      SyntaxKind::Parenthesized => Self::Parenthesized(Parenthesized(node)),
      SyntaxKind::List => Self::List(List(node)),
      SyntaxKind::AttributeSet => Self::AttributeSet(AttributeSet(node)),
      SyntaxKind::LetIn => Self::LetIn(LetIn(node)),
      SyntaxKind::LegacyLet => Self::LegacyLet(LegacyLet(node)),
      SyntaxKind::IfThenElse => Self::IfThenElse(IfThenElse(node)),
      SyntaxKind::Assert => Self::Assert(Assert(node)),
      SyntaxKind::With => Self::With(With(node)),
      SyntaxKind::Lambda => Self::Lambda(Lambda(node)),
      SyntaxKind::Apply => Self::Apply(Apply(node)),
      SyntaxKind::Select => Self::Select(Select(node)),
      SyntaxKind::HasAttribute => Self::HasAttribute(HasAttribute(node)),
      SyntaxKind::UnaryOperation => Self::Unary(UnaryOperation(node)),
      SyntaxKind::BinaryOperation => Self::Binary(BinaryOperation(node)),
      SyntaxKind::ErrorNode => Self::Error(ErrorNode(node)),
      _ => return None,
    })
  }

  fn syntax(self) -> Node<'doc, 'src> {
    match self {
      Self::Literal(node) => node.syntax(),
      Self::String(node) => node.syntax(),
      Self::Path(node) => node.syntax(),
      Self::Parenthesized(node) => node.syntax(),
      Self::List(node) => node.syntax(),
      Self::AttributeSet(node) => node.syntax(),
      Self::LetIn(node) => node.syntax(),
      Self::LegacyLet(node) => node.syntax(),
      Self::IfThenElse(node) => node.syntax(),
      Self::Assert(node) => node.syntax(),
      Self::With(node) => node.syntax(),
      Self::Lambda(node) => node.syntax(),
      Self::Apply(node) => node.syntax(),
      Self::Select(node) => node.syntax(),
      Self::HasAttribute(node) => node.syntax(),
      Self::Unary(node) => node.syntax(),
      Self::Binary(node) => node.syntax(),
      Self::Error(node) => node.syntax(),
    }
  }
}

impl<'doc, 'src> Root<'doc, 'src> {
  /// Returns the root expression.
  #[must_use]
  pub fn expression(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }
}

impl<'doc, 'src> Literal<'doc, 'src> {
  /// Returns the literal token.
  #[must_use]
  pub fn token(self) -> Option<TokenNode<'doc, 'src>> {
    self.0.children().find_map(|element| {
      match element {
        Element::Token(token) if !token.kind().is_trivia() => Some(token),
        Element::Node(_) | Element::Token(_) => None,
      }
    })
  }
}

/// One part of an interpolated string or path.
#[derive(Clone, Copy, Debug)]
pub enum StringPart<'doc, 'src> {
  /// Source text without an interpolation.
  Fragment(TokenNode<'doc, 'src>),
  /// An embedded expression.
  Interpolation(Interpolation<'doc, 'src>),
}

impl<'doc, 'src> StringExpression<'doc, 'src> {
  /// Iterates over raw fragments and interpolations.
  pub fn parts(self) -> impl Iterator<Item = StringPart<'doc, 'src>> {
    string_parts(self.0.children())
  }
}

impl<'doc, 'src> PathExpression<'doc, 'src> {
  /// Iterates over raw fragments and interpolations.
  pub fn parts(self) -> impl Iterator<Item = StringPart<'doc, 'src>> {
    string_parts(self.0.children())
  }
}

impl<'doc, 'src> Interpolation<'doc, 'src> {
  /// Returns the interpolated expression.
  #[must_use]
  pub fn expression(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }
}

impl<'doc, 'src> List<'doc, 'src> {
  /// Iterates over list items.
  pub fn items(self) -> impl Iterator<Item = Expression<'doc, 'src>> {
    expression_children(self.0)
  }
}

impl<'doc, 'src> AttributeSet<'doc, 'src> {
  /// Iterates over bindings and inherit clauses in source order.
  pub fn entries(self) -> impl Iterator<Item = AttributeEntry<'doc, 'src>> {
    self.0.child_nodes().filter_map(AttributeEntry::cast)
  }

  /// Returns whether the set has the `rec` modifier.
  #[must_use]
  pub fn is_recursive(self) -> bool {
    self.0.token(SyntaxKind::RecKeyword).is_some()
  }
}

impl<'doc, 'src> LetIn<'doc, 'src> {
  /// Iterates over local bindings and inherit clauses.
  pub fn entries(self) -> impl Iterator<Item = AttributeEntry<'doc, 'src>> {
    self.0.child_nodes().filter_map(AttributeEntry::cast)
  }

  /// Returns the expression after `in`.
  #[must_use]
  pub fn body(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).last()
  }
}

impl<'doc, 'src> LegacyLet<'doc, 'src> {
  /// Iterates over bindings and inherit clauses.
  pub fn entries(self) -> impl Iterator<Item = AttributeEntry<'doc, 'src>> {
    self.0.child_nodes().filter_map(AttributeEntry::cast)
  }
}

/// A binding or inherit clause inside an attribute set or `let` expression.
#[derive(Clone, Copy, Debug)]
pub enum AttributeEntry<'doc, 'src> {
  /// An `attribute.path = value;` binding.
  Binding(Binding<'doc, 'src>),
  /// An `inherit` clause.
  Inherit(Inherit<'doc, 'src>),
}

impl<'doc, 'src> AttributeEntry<'doc, 'src> {
  fn cast(node: Node<'doc, 'src>) -> Option<Self> {
    match node.kind() {
      SyntaxKind::Binding => Some(Self::Binding(Binding(node))),
      SyntaxKind::Inherit => Some(Self::Inherit(Inherit(node))),
      _ => None,
    }
  }
}

impl<'doc, 'src> Binding<'doc, 'src> {
  /// Returns the bound attribute path.
  #[must_use]
  pub fn path(self) -> Option<AttributePath<'doc, 'src>> {
    self.0.child(SyntaxKind::AttributePath).map(AttributePath)
  }

  /// Returns the bound expression.
  #[must_use]
  pub fn value(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }
}

impl<'doc, 'src> Inherit<'doc, 'src> {
  /// Returns the optional source expression in `inherit (source)`.
  #[must_use]
  pub fn source(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }

  /// Iterates over directly named inherited identifiers.
  pub fn identifiers(self) -> impl Iterator<Item = TokenNode<'doc, 'src>> {
    self.0.children().filter_map(|element| {
      match element {
        Element::Token(token) if token.kind() == SyntaxKind::Identifier => {
          Some(token)
        },
        Element::Node(_) | Element::Token(_) => None,
      }
    })
  }
}

impl<'doc, 'src> AttributePath<'doc, 'src> {
  /// Iterates over the path's static and dynamic components.
  pub fn components(
    self,
  ) -> impl Iterator<Item = AttributeComponent<'doc, 'src>> {
    self.0.children().filter_map(|element| {
      match element {
        Element::Token(token)
          if matches!(
            token.kind(),
            SyntaxKind::Identifier | SyntaxKind::OrKeyword
          ) =>
        {
          Some(AttributeComponent::Identifier(token))
        },
        Element::Node(node) if node.kind() == SyntaxKind::String => {
          Some(AttributeComponent::String(StringExpression(node)))
        },
        Element::Node(node) if node.kind() == SyntaxKind::Interpolation => {
          Some(AttributeComponent::Interpolation(Interpolation(node)))
        },
        Element::Node(_) | Element::Token(_) => None,
      }
    })
  }
}

/// One component of an attribute path.
#[derive(Clone, Copy, Debug)]
pub enum AttributeComponent<'doc, 'src> {
  /// An identifier component.
  Identifier(TokenNode<'doc, 'src>),
  /// A quoted string component.
  String(StringExpression<'doc, 'src>),
  /// A dynamic `${...}` component.
  Interpolation(Interpolation<'doc, 'src>),
}

impl<'doc, 'src> FormalSet<'doc, 'src> {
  /// Iterates over named formal arguments.
  pub fn formals(self) -> impl Iterator<Item = Formal<'doc, 'src>> {
    self.0.child_nodes().filter_map(Formal::cast)
  }

  /// Returns whether the set accepts extra arguments.
  #[must_use]
  pub fn has_ellipsis(self) -> bool {
    self.0.token(SyntaxKind::Ellipsis).is_some()
  }
}

impl<'doc, 'src> Formal<'doc, 'src> {
  /// Returns the formal name.
  #[must_use]
  pub fn name(self) -> Option<TokenNode<'doc, 'src>> {
    self.0.token(SyntaxKind::Identifier)
  }

  /// Returns the default expression, if present.
  #[must_use]
  pub fn default(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }
}

impl<'doc, 'src> Lambda<'doc, 'src> {
  /// Returns the destructured formal set, if any.
  #[must_use]
  pub fn formal_set(self) -> Option<FormalSet<'doc, 'src>> {
    self.0.child(SyntaxKind::FormalSet).map(FormalSet)
  }

  /// Returns the function body.
  #[must_use]
  pub fn body(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).last()
  }

  /// Returns the first simple argument or set-alias identifier.
  #[must_use]
  pub fn argument(self) -> Option<TokenNode<'doc, 'src>> {
    self.0.token(SyntaxKind::Identifier)
  }
}

macro_rules! expression_accessors {
  ($type:ident, $first:ident, $second:ident) => {
    impl<'doc, 'src> $type<'doc, 'src> {
      #[doc = concat!("Returns the ", stringify!($first), " expression.")]
      #[must_use]
      pub fn $first(self) -> Option<Expression<'doc, 'src>> {
        expression_children(self.0).next()
      }

      #[doc = concat!("Returns the ", stringify!($second), " expression.")]
      #[must_use]
      pub fn $second(self) -> Option<Expression<'doc, 'src>> {
        expression_children(self.0).nth(1)
      }
    }
  };
}

expression_accessors!(Apply, function, argument);
expression_accessors!(BinaryOperation, left, right);

impl<'doc, 'src> Select<'doc, 'src> {
  /// Returns the selected expression.
  #[must_use]
  pub fn value(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }

  /// Returns the selected attribute path.
  #[must_use]
  pub fn path(self) -> Option<AttributePath<'doc, 'src>> {
    self.0.child(SyntaxKind::AttributePath).map(AttributePath)
  }

  /// Returns the expression after `or`, if present.
  #[must_use]
  pub fn default(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).nth(1)
  }
}

impl<'doc, 'src> HasAttribute<'doc, 'src> {
  /// Returns the tested expression.
  #[must_use]
  pub fn value(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }

  /// Returns the tested attribute path.
  #[must_use]
  pub fn path(self) -> Option<AttributePath<'doc, 'src>> {
    self.0.child(SyntaxKind::AttributePath).map(AttributePath)
  }
}

impl<'doc, 'src> UnaryOperation<'doc, 'src> {
  /// Returns the operand.
  #[must_use]
  pub fn operand(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }

  /// Returns the unary operator.
  #[must_use]
  pub fn operator(self) -> Option<UnaryOperator> {
    if self.0.token(SyntaxKind::Bang).is_some() {
      Some(UnaryOperator::Not)
    } else if self.0.token(SyntaxKind::Minus).is_some() {
      Some(UnaryOperator::Negate)
    } else {
      None
    }
  }
}

/// A unary Nix operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnaryOperator {
  /// Boolean negation, `!`.
  Not,
  /// Numeric negation, `-`.
  Negate,
}

impl<'doc, 'src> BinaryOperation<'doc, 'src> {
  /// Returns the binary operator.
  #[must_use]
  pub fn operator(self) -> Option<BinaryOperator> {
    self.0.children().find_map(|element| {
      match element {
        Element::Token(token) => BinaryOperator::from_kind(token.kind()),
        Element::Node(_) => None,
      }
    })
  }
}

/// A binary Nix operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryOperator {
  /// Equality, `==`.
  Equal,
  /// Inequality, `!=`.
  NotEqual,
  /// Less than, `<`.
  Less,
  /// Greater than, `>`.
  Greater,
  /// Less than or equal, `<=`.
  LessEqual,
  /// Greater than or equal, `>=`.
  GreaterEqual,
  /// Boolean conjunction, `&&`.
  And,
  /// Boolean disjunction, `||`.
  Or,
  /// Boolean implication, `->`.
  Implication,
  /// Attribute-set update, `//`.
  Update,
  /// Addition, `+`.
  Add,
  /// Subtraction, `-`.
  Subtract,
  /// Multiplication, `*`.
  Multiply,
  /// Division, `/`.
  Divide,
  /// List concatenation, `++`.
  Concatenate,
  /// Right pipe, `|>`.
  PipeInto,
  /// Left pipe, `<|`.
  PipeFrom,
}

impl BinaryOperator {
  fn from_kind(kind: SyntaxKind) -> Option<Self> {
    Some(match kind {
      SyntaxKind::Equal => Self::Equal,
      SyntaxKind::NotEqual => Self::NotEqual,
      SyntaxKind::Less => Self::Less,
      SyntaxKind::Greater => Self::Greater,
      SyntaxKind::LessEqual => Self::LessEqual,
      SyntaxKind::GreaterEqual => Self::GreaterEqual,
      SyntaxKind::And => Self::And,
      SyntaxKind::Or => Self::Or,
      SyntaxKind::Implication => Self::Implication,
      SyntaxKind::Update => Self::Update,
      SyntaxKind::Plus => Self::Add,
      SyntaxKind::Minus => Self::Subtract,
      SyntaxKind::Star => Self::Multiply,
      SyntaxKind::Slash => Self::Divide,
      SyntaxKind::Concatenate => Self::Concatenate,
      SyntaxKind::PipeInto => Self::PipeInto,
      SyntaxKind::PipeFrom => Self::PipeFrom,
      _ => return None,
    })
  }
}

impl<'doc, 'src> IfThenElse<'doc, 'src> {
  /// Returns the condition.
  #[must_use]
  pub fn condition(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }

  /// Returns the expression after `then`.
  #[must_use]
  pub fn then_branch(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).nth(1)
  }

  /// Returns the expression after `else`.
  #[must_use]
  pub fn else_branch(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).nth(2)
  }
}

expression_accessors!(Assert, condition, body);
expression_accessors!(With, scope, body);

impl<'doc, 'src> Parenthesized<'doc, 'src> {
  /// Returns the inner expression.
  #[must_use]
  pub fn expression(self) -> Option<Expression<'doc, 'src>> {
    expression_children(self.0).next()
  }
}

fn expression_children<'doc, 'src>(
  node: Node<'doc, 'src>,
) -> impl Iterator<Item = Expression<'doc, 'src>> {
  node.child_nodes().filter_map(Expression::cast)
}

fn string_parts<'doc, 'src>(
  children: Children<'doc, 'src>,
) -> impl Iterator<Item = StringPart<'doc, 'src>> {
  children.filter_map(|element| {
    match element {
      Element::Token(token)
        if matches!(
          token.kind(),
          SyntaxKind::StringFragment | SyntaxKind::PathFragment
        ) =>
      {
        Some(StringPart::Fragment(token))
      },
      Element::Node(node) if node.kind() == SyntaxKind::Interpolation => {
        Some(StringPart::Interpolation(Interpolation(node)))
      },
      Element::Node(_) | Element::Token(_) => None,
    }
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parse;

  #[test]
  fn traverses_typed_expression() {
    let document = parse("{ x, ... }: [ x 2 ]").expect("small source");
    let root = Root::cast(document.root()).expect("root node");
    let Expression::Lambda(lambda) =
      root.expression().expect("root expression")
    else {
      panic!("expected lambda")
    };
    let formals = lambda.formal_set().expect("formal set");
    assert!(formals.has_ellipsis());
    assert_eq!(
      formals
        .formals()
        .next()
        .and_then(Formal::name)
        .map(TokenNode::text),
      Some("x")
    );
    let Expression::List(list) = lambda.body().expect("lambda body") else {
      panic!("expected list")
    };
    assert_eq!(list.items().count(), 2);
  }
}
