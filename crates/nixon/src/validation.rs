use std::{
  borrow::Cow,
  collections::HashMap,
};

use crate::{
  Diagnostic,
  DiagnosticKind,
  Document,
  Element,
  ElementId,
  Node,
  ParseOptions,
  Severity,
  SyntaxKind,
  TextRange,
  UrlLiteralPolicy,
  ast::{
    AstNode,
    AttributeComponent,
    AttributeEntry,
    AttributePath,
    AttributeSet,
    BinaryOperation,
    Expression,
    FormalSet,
    HasAttribute,
    Lambda,
    LegacyLet,
    LetIn,
    Select,
    StringPart,
  },
};

pub(crate) fn validate(
  document: &Document<'_>,
  options: ParseOptions<'_>,
) -> Vec<Diagnostic> {
  let mut diagnostics = Vec::new();
  validate_tokens(document, options, &mut diagnostics);
  validate_nodes(document, &mut diagnostics);
  if options.validate_identifiers {
    validate_identifiers(document, options, &mut diagnostics);
  }
  diagnostics
}

fn validate_tokens(
  document: &Document<'_>,
  options: ParseOptions<'_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  for id in 0..document.element_count() {
    let Some(Element::Token(token)) =
      document.element(ElementId::new(id as u32))
    else {
      continue;
    };
    let (kind, severity) = match token.kind() {
      SyntaxKind::PipeFrom | SyntaxKind::PipeInto
        if !options.pipe_operators =>
      {
        (DiagnosticKind::ExperimentalFeatureDisabled, Severity::Error)
      },
      SyntaxKind::Uri if options.uri_literals != UrlLiteralPolicy::Allow => {
        (
          DiagnosticKind::UriLiteral,
          if options.uri_literals == UrlLiteralPolicy::Deny {
            Severity::Error
          } else {
            Severity::Warning
          },
        )
      },
      SyntaxKind::Integer if token.text().parse::<i64>().is_err() => {
        (DiagnosticKind::IntegerOverflow, Severity::Error)
      },
      SyntaxKind::Float if !is_finite_float(token.text()) => {
        (DiagnosticKind::FloatOutOfRange, Severity::Error)
      },
      SyntaxKind::Path if token.text().ends_with('/') => {
        (DiagnosticKind::TrailingSlashPath, Severity::Error)
      },
      _ => continue,
    };
    diagnostics.push(Diagnostic::validation(
      kind,
      severity,
      token.range(),
      token.kind(),
    ));
  }
}

fn validate_nodes(document: &Document<'_>, diagnostics: &mut Vec<Diagnostic>) {
  for id in 0..document.element_count() {
    let Some(Element::Node(node)) = document.element(ElementId::new(id as u32))
    else {
      continue;
    };
    match node.kind() {
      SyntaxKind::FormalSet => {
        validate_formals(
          FormalSet::cast(node).expect("kind checked"),
          diagnostics,
        );
      },
      SyntaxKind::Lambda => {
        validate_lambda(Lambda::cast(node).expect("kind checked"), diagnostics);
      },
      SyntaxKind::AttributeSet if !is_binding_value(node) => {
        validate_entries(
          AttributeSet::cast(node).expect("kind checked").entries(),
          false,
          diagnostics,
        );
      },
      SyntaxKind::LetIn => {
        validate_entries(
          LetIn::cast(node).expect("kind checked").entries(),
          true,
          diagnostics,
        )
      },
      SyntaxKind::LegacyLet => {
        validate_entries(
          LegacyLet::cast(node).expect("kind checked").entries(),
          false,
          diagnostics,
        )
      },
      _ => {},
    }
  }
}

/// Returns whether `node` is the value of a binding, possibly parenthesized.
///
/// Nix merges such sets into the enclosing set, so their entries are checked
/// together with the enclosing bindings.
fn is_binding_value(node: Node<'_, '_>) -> bool {
  let mut current = node;
  while let Some(parent) = current.parent() {
    match parent.kind() {
      SyntaxKind::Binding => return true,
      SyntaxKind::Parenthesized => current = parent,
      _ => return false,
    }
  }
  false
}

fn validate_formals(
  formals: FormalSet<'_, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let mut names = Vec::new();
  for formal in formals.formals() {
    let Some(name) = formal.name() else {
      continue;
    };
    if names.contains(&name.text()) {
      diagnostics.push(Diagnostic::validation(
        DiagnosticKind::DuplicateFormal,
        Severity::Error,
        name.range(),
        name.kind(),
      ));
    } else {
      names.push(name.text());
    }
  }
}

fn validate_lambda(lambda: Lambda<'_, '_>, diagnostics: &mut Vec<Diagnostic>) {
  let (Some(alias), Some(formals)) = (lambda.argument(), lambda.formal_set())
  else {
    return;
  };
  if formals
    .formals()
    .filter_map(|formal| formal.name())
    .any(|name| name.text() == alias.text())
  {
    diagnostics.push(Diagnostic::validation(
      DiagnosticKind::DuplicateFormal,
      Severity::Error,
      alias.range(),
      alias.kind(),
    ));
  }
}

fn validate_entries<'doc, 'src: 'doc>(
  entries: impl Iterator<Item = AttributeEntry<'doc, 'src>>,
  is_let: bool,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let mut paths = PathTrie::default();
  define_entries(entries, &[], is_let, &mut paths, diagnostics);
}

/// What a definition binds at its static attribute path.
#[derive(Clone, Copy)]
enum Shape {
  /// A value Nix cannot merge with other definitions.
  Value,
  /// An attribute set Nix merges with other attribute-set definitions.
  Set,
}

/// Records the static definitions of a set of entries in source order.
///
/// This mirrors Nix's `addAttr`: every static path component before the last
/// one defines a nested attribute set, a binding whose value is an attribute
/// set literal is merged into its path, and the rest of a path after a dynamic
/// component lands in a fresh set that Nix does not check for duplicates.
fn define_entries<'doc, 'src: 'doc>(
  entries: impl Iterator<Item = AttributeEntry<'doc, 'src>>,
  prefix: &[Cow<'src, str>],
  is_let: bool,
  paths: &mut PathTrie<'src>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  for entry in entries {
    match entry {
      AttributeEntry::Binding(binding) => {
        let Some(path) = binding.path() else {
          continue;
        };
        let range = path.syntax().range();
        let mut full_path = prefix.to_vec();
        let mut dynamic = false;
        for (index, component) in path.components().enumerate() {
          if let Some(name) = component.static_name() {
            full_path.push(name);
            continue;
          }
          // Nix rejects only a dynamic first component in `let`; later ones
          // land in a nested set, which may be dynamic.
          if is_let && index == 0 {
            diagnostics.push(Diagnostic::validation(
              DiagnosticKind::DynamicAttributeInLet,
              Severity::Error,
              range,
              SyntaxKind::AttributePath,
            ));
          }
          dynamic = true;
          break;
        }
        if full_path.len() == prefix.len() {
          continue;
        }
        let set = if dynamic {
          None
        } else if let Some(Expression::AttributeSet(set)) =
          binding.value().map(unparenthesize)
        {
          Some(set)
        } else {
          define(paths, full_path, range, Shape::Value, diagnostics);
          continue;
        };
        let defined =
          define(paths, full_path.clone(), range, Shape::Set, diagnostics);
        if defined && let Some(set) = set {
          define_entries(set.entries(), &full_path, false, paths, diagnostics);
        }
      },
      AttributeEntry::Inherit(inherit) => {
        for component in inherit.attributes() {
          let Some(name) = component.static_name() else {
            diagnostics.push(Diagnostic::validation(
              DiagnosticKind::DynamicAttributeInInherit,
              Severity::Error,
              component.range(),
              component_kind(component),
            ));
            continue;
          };
          let mut full_path = prefix.to_vec();
          full_path.push(name);
          define(
            paths,
            full_path,
            component.range(),
            Shape::Value,
            diagnostics,
          );
        }
      },
    }
  }
}

/// Records one definition and reports whether it was accepted.
fn define<'src>(
  paths: &mut PathTrie<'src>,
  path: Vec<Cow<'src, str>>,
  range: TextRange,
  shape: Shape,
  diagnostics: &mut Vec<Diagnostic>,
) -> bool {
  let Some(kind) = paths.insert(path, shape) else {
    return true;
  };
  diagnostics.push(Diagnostic::validation(
    kind,
    Severity::Error,
    range,
    SyntaxKind::AttributePath,
  ));
  false
}

fn unparenthesize<'doc, 'src>(
  mut expression: Expression<'doc, 'src>,
) -> Expression<'doc, 'src> {
  while let Expression::Parenthesized(parenthesized) = expression
    && let Some(inner) = parenthesized.expression()
  {
    expression = inner;
  }
  expression
}

fn component_kind(component: AttributeComponent<'_, '_>) -> SyntaxKind {
  match component {
    AttributeComponent::Identifier(token) => token.kind(),
    AttributeComponent::String(string) => string.syntax().kind(),
    AttributeComponent::Interpolation(interpolation) => {
      interpolation.syntax().kind()
    },
  }
}

#[derive(Default)]
struct PathTrie<'src> {
  value:    bool,
  set:      bool,
  children: HashMap<Cow<'src, str>, Self>,
}

impl<'src> PathTrie<'src> {
  fn insert(
    &mut self,
    path: Vec<Cow<'src, str>>,
    shape: Shape,
  ) -> Option<DiagnosticKind> {
    let mut current = self;
    for component in path {
      if current.value {
        return Some(DiagnosticKind::ConflictingAttribute);
      }
      current.set = true;
      current = current.children.entry(component).or_default();
    }
    match shape {
      Shape::Value if current.value => Some(DiagnosticKind::DuplicateAttribute),
      Shape::Value if current.set => Some(DiagnosticKind::ConflictingAttribute),
      Shape::Set if current.value => Some(DiagnosticKind::ConflictingAttribute),
      Shape::Value => {
        current.value = true;
        None
      },
      Shape::Set => {
        current.set = true;
        None
      },
    }
  }
}

fn is_finite_float(text: &str) -> bool {
  match text.parse::<f64>() {
    Ok(value) => value.is_finite(),
    Err(_) => false,
  }
}

fn validate_identifiers(
  document: &Document<'_>,
  options: ParseOptions<'_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let Some(expression) =
    crate::ast::Root::cast(document.root()).and_then(|root| root.expression())
  else {
    return;
  };
  let mut scope = Scope {
    frames:             Vec::new(),
    dynamic:            0,
    depth:              0,
    additional_globals: options.additional_globals,
  };
  validate_expression(expression, &mut scope, diagnostics);
}

struct Scope<'src, 'options> {
  frames:             Vec<Vec<Cow<'src, str>>>,
  dynamic:            usize,
  depth:              u16,
  additional_globals: &'options [&'options str],
}

impl Scope<'_, '_> {
  fn contains(&self, name: &str) -> bool {
    self.dynamic > 0
      || self
        .frames
        .iter()
        .rev()
        .any(|frame| frame.iter().any(|candidate| candidate == name))
      || BASE_GLOBALS.binary_search(&name).is_ok()
      || self.additional_globals.contains(&name)
  }
}

fn validate_expression<'doc, 'src: 'doc>(
  expression: Expression<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if scope.depth == crate::MAX_NESTING_DEPTH {
    diagnostics.push(Diagnostic::validation(
      DiagnosticKind::NestingLimit,
      Severity::Error,
      expression.syntax().range(),
      expression.syntax().kind(),
    ));
    return;
  }
  scope.depth += 1;
  let frames = scope.frames.len();
  let dynamic = scope.dynamic;
  // Walk tail-position chains (`assert`, `with`, `let ... in`, lambda bodies,
  // `else` branches) in a loop, as the parser does, so they share one level.
  let mut next = Some(expression);
  while let Some(expression) = next {
    next = validate_tail_head(expression, scope, diagnostics);
  }
  scope.frames.truncate(frames);
  scope.dynamic = dynamic;
  scope.depth -= 1;
}

/// Validates an expression except for its tail expression, which it returns.
///
/// Scopes the tail is evaluated in are left on `scope` for the caller to
/// remove. Expressions without a tail are validated completely.
fn validate_tail_head<'doc, 'src: 'doc>(
  expression: Expression<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) -> Option<Expression<'doc, 'src>> {
  match expression {
    Expression::Assert(assertion) => {
      if let Some(condition) = assertion.condition() {
        validate_expression(condition, scope, diagnostics);
      }
      assertion.body()
    },
    Expression::With(with) => {
      if let Some(scope_expression) = with.scope() {
        validate_expression(scope_expression, scope, diagnostics);
      }
      scope.dynamic += 1;
      with.body()
    },
    Expression::LetIn(let_in) => {
      scope.frames.push(collect_entry_names(let_in.entries()));
      for entry in let_in.entries() {
        validate_entry(entry, scope, diagnostics);
      }
      let_in.body()
    },
    Expression::Lambda(lambda) => {
      enter_function(lambda, scope, diagnostics);
      lambda.body()
    },
    Expression::IfThenElse(conditional) => {
      for expression in [conditional.condition(), conditional.then_branch()]
        .into_iter()
        .flatten()
      {
        validate_expression(expression, scope, diagnostics);
      }
      conditional.else_branch()
    },
    _ => {
      validate_expression_inner(expression, scope, diagnostics);
      None
    },
  }
}

fn validate_expression_inner<'doc, 'src: 'doc>(
  expression: Expression<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  match expression {
    Expression::Literal(literal) => {
      let Some(token) = literal.token() else {
        return;
      };
      if token.kind() == SyntaxKind::Identifier && !scope.contains(token.text())
      {
        diagnostics.push(Diagnostic::validation(
          DiagnosticKind::UndefinedVariable,
          Severity::Error,
          token.range(),
          token.kind(),
        ));
      }
    },
    Expression::String(string) => {
      for part in string.parts() {
        if let StringPart::Interpolation(interpolation) = part
          && let Some(expression) = interpolation.expression()
        {
          validate_expression(expression, scope, diagnostics);
        }
      }
    },
    Expression::Path(path) => {
      for part in path.parts() {
        if let StringPart::Interpolation(interpolation) = part
          && let Some(expression) = interpolation.expression()
        {
          validate_expression(expression, scope, diagnostics);
        }
      }
    },
    Expression::Parenthesized(parenthesized) => {
      if let Some(expression) = parenthesized.expression() {
        validate_expression(expression, scope, diagnostics);
      }
    },
    Expression::List(list) => {
      for item in list.items() {
        validate_expression(item, scope, diagnostics);
      }
    },
    Expression::AttributeSet(set) => {
      validate_attribute_set(set, scope, diagnostics)
    },
    Expression::LegacyLet(legacy) => {
      validate_legacy_let(legacy, scope, diagnostics)
    },
    Expression::Apply(apply) => {
      validate_pair(apply.function(), apply.argument(), scope, diagnostics);
    },
    Expression::Select(select) => validate_select(select, scope, diagnostics),
    Expression::HasAttribute(test) => {
      validate_has_attribute(test, scope, diagnostics)
    },
    Expression::Unary(operation) => {
      if let Some(operand) = operation.operand() {
        validate_expression(operand, scope, diagnostics);
      }
    },
    Expression::Binary(operation) => {
      validate_binary(operation, scope, diagnostics);
    },
    // Constructs with a tail expression are handled by `validate_tail_head`.
    Expression::Error(_)
    | Expression::LetIn(_)
    | Expression::IfThenElse(_)
    | Expression::Assert(_)
    | Expression::With(_)
    | Expression::Lambda(_) => {},
  }
}

fn validate_pair<'doc, 'src: 'doc>(
  first: Option<Expression<'doc, 'src>>,
  second: Option<Expression<'doc, 'src>>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  for expression in [first, second].into_iter().flatten() {
    validate_expression(expression, scope, diagnostics);
  }
}

fn validate_binary<'doc, 'src: 'doc>(
  operation: BinaryOperation<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  validate_pair(operation.left(), operation.right(), scope, diagnostics);
}

fn validate_select<'doc, 'src: 'doc>(
  select: Select<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if let Some(value) = select.value() {
    validate_expression(value, scope, diagnostics);
  }
  if let Some(path) = select.path() {
    validate_attribute_path(path, scope, diagnostics);
  }
  if let Some(default) = select.default() {
    validate_expression(default, scope, diagnostics);
  }
}

fn validate_has_attribute<'doc, 'src: 'doc>(
  test: HasAttribute<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if let Some(value) = test.value() {
    validate_expression(value, scope, diagnostics);
  }
  if let Some(path) = test.path() {
    validate_attribute_path(path, scope, diagnostics);
  }
}

fn validate_attribute_path<'doc, 'src: 'doc>(
  path: AttributePath<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  for component in path.components() {
    match component {
      AttributeComponent::String(string) => {
        for part in string.parts() {
          if let StringPart::Interpolation(interpolation) = part
            && let Some(expression) = interpolation.expression()
          {
            validate_expression(expression, scope, diagnostics);
          }
        }
      },
      AttributeComponent::Interpolation(interpolation) => {
        if let Some(expression) = interpolation.expression() {
          validate_expression(expression, scope, diagnostics);
        }
      },
      AttributeComponent::Identifier(_) => {},
    }
  }
}

/// Pushes a function's argument scope and validates its formal defaults.
fn enter_function<'doc, 'src: 'doc>(
  lambda: Lambda<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let mut frame = Vec::new();
  if let Some(argument) = lambda.argument() {
    frame.push(Cow::Borrowed(argument.text()));
  }
  if let Some(formals) = lambda.formal_set() {
    frame.extend(
      formals
        .formals()
        .filter_map(|formal| formal.name())
        .map(|name| Cow::Borrowed(name.text())),
    );
  }
  scope.frames.push(frame);
  if let Some(formals) = lambda.formal_set() {
    for default in formals.formals().filter_map(|formal| formal.default()) {
      validate_expression(default, scope, diagnostics);
    }
  }
}

fn validate_legacy_let<'doc, 'src: 'doc>(
  legacy: LegacyLet<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  let frame = collect_entry_names(legacy.entries());
  scope.frames.push(frame);
  for entry in legacy.entries() {
    validate_entry(entry, scope, diagnostics);
  }
  scope.frames.pop();
}

fn validate_attribute_set<'doc, 'src: 'doc>(
  set: AttributeSet<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  if set.is_recursive() {
    scope.frames.push(collect_entry_names(set.entries()));
  }
  for entry in set.entries() {
    validate_entry(entry, scope, diagnostics);
  }
  if set.is_recursive() {
    scope.frames.pop();
  }
}

fn validate_entry<'doc, 'src: 'doc>(
  entry: AttributeEntry<'doc, 'src>,
  scope: &mut Scope<'src, '_>,
  diagnostics: &mut Vec<Diagnostic>,
) {
  match entry {
    AttributeEntry::Binding(binding) => {
      if let Some(path) = binding.path() {
        validate_attribute_path(path, scope, diagnostics);
      }
      if let Some(value) = binding.value() {
        validate_expression(value, scope, diagnostics);
      }
    },
    AttributeEntry::Inherit(inherit) => {
      if let Some(source) = inherit.source() {
        validate_expression(source, scope, diagnostics);
        return;
      }
      for component in inherit.attributes() {
        if let Some(name) = component.static_name()
          && !scope.contains(&name)
        {
          diagnostics.push(Diagnostic::validation(
            DiagnosticKind::UndefinedVariable,
            Severity::Error,
            component.range(),
            component_kind(component),
          ));
        }
      }
    },
  }
}

fn collect_entry_names<'doc, 'src: 'doc>(
  entries: impl Iterator<Item = AttributeEntry<'doc, 'src>>,
) -> Vec<Cow<'src, str>> {
  let mut names = Vec::new();
  for entry in entries {
    match entry {
      AttributeEntry::Binding(binding) => {
        if let Some(name) = binding
          .path()
          .and_then(|path| path.components().next())
          .and_then(AttributeComponent::static_name)
        {
          names.push(name);
        }
      },
      AttributeEntry::Inherit(inherit) => {
        names.extend(
          inherit
            .attributes()
            .filter_map(AttributeComponent::static_name),
        );
      },
    }
  }
  names
}

/// Names in the base static scope of Nix 2.36, sorted for binary search.
///
/// These are the identifiers every Nix expression may reference without a
/// binding: the constants, the builtins exposed at top level, and their
/// `__`-prefixed aliases. Names that Nix only defines with an experimental
/// feature or setting enabled (flakes, `fetch-closure`, unsafe native code)
/// are included, since the parser cannot know the evaluator's settings.
pub const BASE_GLOBALS: &[&str] = &[
  "__add",
  "__addDrvOutputDependencies",
  "__addErrorContext",
  "__all",
  "__any",
  "__appendContext",
  "__attrNames",
  "__attrValues",
  "__bitAnd",
  "__bitOr",
  "__bitXor",
  "__catAttrs",
  "__ceil",
  "__compareVersions",
  "__concatLists",
  "__concatMap",
  "__concatStringsSep",
  "__convertHash",
  "__curPos",
  "__currentSystem",
  "__currentTime",
  "__deepSeq",
  "__div",
  "__elem",
  "__elemAt",
  "__exec",
  "__fetchClosure",
  "__fetchurl",
  "__filter",
  "__filterSource",
  "__findFile",
  "__flakeRefToString",
  "__floor",
  "__foldl'",
  "__fromJSON",
  "__functionArgs",
  "__genList",
  "__genericClosure",
  "__getAttr",
  "__getContext",
  "__getEnv",
  "__getFlake",
  "__groupBy",
  "__hasAttr",
  "__hasContext",
  "__hashFile",
  "__hashString",
  "__head",
  "__importNative",
  "__intersectAttrs",
  "__isAttrs",
  "__isBool",
  "__isFloat",
  "__isFunction",
  "__isInt",
  "__isList",
  "__isPath",
  "__isString",
  "__langVersion",
  "__length",
  "__lessThan",
  "__listToAttrs",
  "__mapAttrs",
  "__match",
  "__mul",
  "__nixPath",
  "__nixVersion",
  "__outputOf",
  "__parseDrvName",
  "__parseFlakeRef",
  "__partition",
  "__path",
  "__pathExists",
  "__readDir",
  "__readFile",
  "__readFileType",
  "__replaceStrings",
  "__seq",
  "__sort",
  "__split",
  "__splitVersion",
  "__storeDir",
  "__storePath",
  "__stringLength",
  "__sub",
  "__substring",
  "__tail",
  "__toFile",
  "__toJSON",
  "__toPath",
  "__toXML",
  "__trace",
  "__traceVerbose",
  "__tryEval",
  "__typeOf",
  "__unsafeDiscardOutputDependency",
  "__unsafeDiscardStringContext",
  "__unsafeGetAttrPos",
  "__warn",
  "__zipAttrsWith",
  "abort",
  "baseNameOf",
  "break",
  "builtins",
  "derivation",
  "derivationStrict",
  "dirOf",
  "false",
  "fetchGit",
  "fetchMercurial",
  "fetchTarball",
  "fetchTree",
  "fromTOML",
  "import",
  "isNull",
  "map",
  "null",
  "placeholder",
  "removeAttrs",
  "scopedImport",
  "throw",
  "toString",
  "true",
];

#[cfg(test)]
mod tests {
  use super::BASE_GLOBALS;
  use crate::{
    DiagnosticKind,
    ParseOptions,
    UrlLiteralPolicy,
    parse,
    parse_with_options,
  };

  fn has(source: &str, kind: DiagnosticKind) -> bool {
    parse(source)
      .expect("small source")
      .diagnostics()
      .iter()
      .any(|diagnostic| diagnostic.kind() == kind)
  }

  #[test]
  fn checks_formals_and_attribute_paths() {
    assert!(has("{ x, x }: x", DiagnosticKind::DuplicateFormal));
    assert!(has("x@{ x }: x", DiagnosticKind::DuplicateFormal));
    assert!(has("{ a = 1; a = 2; }", DiagnosticKind::DuplicateAttribute));
    assert!(has(
      "{ a = 1; a.b = 2; }",
      DiagnosticKind::ConflictingAttribute
    ));
  }

  #[test]
  fn checks_numeric_and_path_limits() {
    assert!(has("9223372036854775808", DiagnosticKind::IntegerOverflow));
    assert!(has("1.0e999", DiagnosticKind::FloatOutOfRange));
    assert!(has("./foo/", DiagnosticKind::TrailingSlashPath));
  }

  #[test]
  fn honors_feature_and_uri_options() {
    assert!(has("x |> f", DiagnosticKind::ExperimentalFeatureDisabled));
    let options = ParseOptions {
      pipe_operators: true,
      uri_literals: UrlLiteralPolicy::Allow,
      validate_identifiers: false,
      ..ParseOptions::default()
    };
    assert!(
      parse_with_options("x |> f", options)
        .expect("small source")
        .is_valid()
    );
  }

  #[test]
  fn resolves_static_scopes() {
    assert!(has("missing", DiagnosticKind::UndefinedVariable));
    assert!(
      parse("let x = y; y = 1; in x")
        .expect("small source")
        .is_valid()
    );
    assert!(parse("{ x, y ? x }: y").expect("small source").is_valid());
    assert!(
      parse("with builtins; unknownName")
        .expect("small source")
        .is_valid()
    );
    assert!(has(
      "with missing; unknownName",
      DiagnosticKind::UndefinedVariable
    ));
  }

  #[test]
  fn base_globals_are_sorted() {
    assert!(BASE_GLOBALS.windows(2).all(|pair| pair[0] < pair[1]));
  }
}
