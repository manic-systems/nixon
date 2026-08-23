use std::borrow::Cow;

use crate::{
    Diagnostic, DiagnosticKind, Document, Element, ElementId, ParseOptions, Severity, SyntaxKind,
    UrlLiteralPolicy,
    ast::{
        AstNode, AttributeComponent, AttributeEntry, AttributePath, AttributeSet, BinaryOperation,
        Expression, FormalSet, HasAttribute, Lambda, LegacyLet, LetIn, Select, StringPart,
    },
};

pub(crate) fn validate(document: &Document<'_>, options: ParseOptions<'_>) -> Vec<Diagnostic> {
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
        let Some(Element::Token(token)) = document.element(ElementId::new(id as u32)) else {
            continue;
        };
        let (kind, severity) = match token.kind() {
            SyntaxKind::PipeFrom | SyntaxKind::PipeInto if !options.pipe_operators => {
                (DiagnosticKind::ExperimentalFeatureDisabled, Severity::Error)
            }
            SyntaxKind::Uri if options.uri_literals != UrlLiteralPolicy::Allow => (
                DiagnosticKind::UriLiteral,
                if options.uri_literals == UrlLiteralPolicy::Deny {
                    Severity::Error
                } else {
                    Severity::Warning
                },
            ),
            SyntaxKind::Integer if token.text().parse::<i64>().is_err() => {
                (DiagnosticKind::IntegerOverflow, Severity::Error)
            }
            SyntaxKind::Float if !is_finite_float(token.text()) => {
                (DiagnosticKind::FloatOutOfRange, Severity::Error)
            }
            SyntaxKind::Path if token.text().ends_with('/') => {
                (DiagnosticKind::TrailingSlashPath, Severity::Error)
            }
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
        let Some(Element::Node(node)) = document.element(ElementId::new(id as u32)) else {
            continue;
        };
        match node.kind() {
            SyntaxKind::FormalSet => {
                validate_formals(FormalSet::cast(node).expect("kind checked"), diagnostics);
            }
            SyntaxKind::Lambda => {
                validate_lambda(Lambda::cast(node).expect("kind checked"), diagnostics);
            }
            SyntaxKind::AttributeSet
                if node
                    .parent()
                    .is_none_or(|parent| parent.kind() != SyntaxKind::Binding) =>
            {
                validate_entries(
                    AttributeSet::cast(node).expect("kind checked").entries(),
                    false,
                    diagnostics,
                );
            }
            SyntaxKind::LetIn => validate_entries(
                LetIn::cast(node).expect("kind checked").entries(),
                true,
                diagnostics,
            ),
            SyntaxKind::LegacyLet => validate_entries(
                LegacyLet::cast(node).expect("kind checked").entries(),
                false,
                diagnostics,
            ),
            _ => {}
        }
    }
}

fn validate_formals(formals: FormalSet<'_, '_>, diagnostics: &mut Vec<Diagnostic>) {
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
    let (Some(alias), Some(formals)) = (lambda.argument(), lambda.formal_set()) else {
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
    let mut leaves = Vec::new();
    collect_leaf_paths(entries, &[], is_let, diagnostics, &mut leaves);
    let mut paths = Vec::new();
    for (path, range) in leaves {
        check_path(&mut paths, path, range, diagnostics);
    }
}

fn collect_leaf_paths<'doc, 'src: 'doc>(
    entries: impl Iterator<Item = AttributeEntry<'doc, 'src>>,
    prefix: &[String],
    is_let: bool,
    diagnostics: &mut Vec<Diagnostic>,
    leaves: &mut Vec<(Vec<String>, crate::TextRange)>,
) {
    for entry in entries {
        match entry {
            AttributeEntry::Binding(binding) => {
                let Some(path) = binding.path() else {
                    continue;
                };
                let Some(components) = static_path(path) else {
                    if is_let {
                        diagnostics.push(Diagnostic::validation(
                            DiagnosticKind::DynamicAttributeInLet,
                            Severity::Error,
                            path.syntax().range(),
                            SyntaxKind::AttributePath,
                        ));
                    }
                    continue;
                };
                let mut full_path = Vec::with_capacity(prefix.len() + components.len());
                full_path.extend_from_slice(prefix);
                full_path.extend(components);
                if let Some(Expression::AttributeSet(set)) = binding.value() {
                    collect_leaf_paths(set.entries(), &full_path, false, diagnostics, leaves);
                } else {
                    leaves.push((full_path, path.syntax().range()));
                }
            }
            AttributeEntry::Inherit(inherit) => {
                for name in inherit.identifiers() {
                    let mut full_path = Vec::with_capacity(prefix.len() + 1);
                    full_path.extend_from_slice(prefix);
                    full_path.push(name.text().to_owned());
                    leaves.push((full_path, name.range()));
                }
                if inherit
                    .syntax()
                    .child_nodes()
                    .any(|node| node.kind() == SyntaxKind::Interpolation)
                {
                    diagnostics.push(Diagnostic::validation(
                        DiagnosticKind::DynamicAttributeInInherit,
                        Severity::Error,
                        inherit.syntax().range(),
                        SyntaxKind::Interpolation,
                    ));
                }
            }
        }
    }
}

fn static_path(path: AttributePath<'_, '_>) -> Option<Vec<String>> {
    path.components()
        .map(|component| match component {
            AttributeComponent::Identifier(token) => Some(token.text().to_owned()),
            AttributeComponent::String(string) => static_string(string.syntax().text()),
            AttributeComponent::Interpolation(_) => None,
        })
        .collect()
}

fn static_string(text: &str) -> Option<String> {
    let body = text.strip_prefix('"')?.strip_suffix('"')?;
    if body.contains("${") {
        return None;
    }
    let mut result = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(character) = chars.next() {
        if character == '\\' {
            let escaped = chars.next()?;
            result.push(match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
        } else {
            result.push(character);
        }
    }
    Some(result)
}

fn check_path(
    paths: &mut Vec<Vec<String>>,
    path: Vec<String>,
    range: crate::TextRange,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for previous in paths.iter() {
        let kind = if previous == &path {
            Some(DiagnosticKind::DuplicateAttribute)
        } else if is_prefix(previous, &path) || is_prefix(&path, previous) {
            Some(DiagnosticKind::ConflictingAttribute)
        } else {
            None
        };
        if let Some(kind) = kind {
            diagnostics.push(Diagnostic::validation(
                kind,
                Severity::Error,
                range,
                SyntaxKind::AttributePath,
            ));
            return;
        }
    }
    paths.push(path);
}

fn is_prefix(left: &[String], right: &[String]) -> bool {
    left.len() < right.len() && left.iter().zip(right).all(|(left, right)| left == right)
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
        frames: Vec::new(),
        dynamic: 0,
        additional_globals: options.additional_globals,
    };
    validate_expression(expression, &mut scope, diagnostics);
}

struct Scope<'src, 'options> {
    frames: Vec<Vec<Cow<'src, str>>>,
    dynamic: usize,
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
    match expression {
        Expression::Literal(literal) => {
            let Some(token) = literal.token() else {
                return;
            };
            if token.kind() == SyntaxKind::Identifier && !scope.contains(token.text()) {
                diagnostics.push(Diagnostic::validation(
                    DiagnosticKind::UndefinedVariable,
                    Severity::Error,
                    token.range(),
                    token.kind(),
                ));
            }
        }
        Expression::String(string) => {
            for part in string.parts() {
                if let StringPart::Interpolation(interpolation) = part
                    && let Some(expression) = interpolation.expression()
                {
                    validate_expression(expression, scope, diagnostics);
                }
            }
        }
        Expression::Path(path) => {
            for part in path.parts() {
                if let StringPart::Interpolation(interpolation) = part
                    && let Some(expression) = interpolation.expression()
                {
                    validate_expression(expression, scope, diagnostics);
                }
            }
        }
        Expression::Parenthesized(parenthesized) => {
            if let Some(expression) = parenthesized.expression() {
                validate_expression(expression, scope, diagnostics);
            }
        }
        Expression::List(list) => {
            for item in list.items() {
                validate_expression(item, scope, diagnostics);
            }
        }
        Expression::AttributeSet(set) => validate_attribute_set(set, scope, diagnostics),
        Expression::LetIn(let_in) => validate_let(let_in, scope, diagnostics),
        Expression::LegacyLet(legacy) => validate_legacy_let(legacy, scope, diagnostics),
        Expression::IfThenElse(conditional) => {
            for expression in [
                conditional.condition(),
                conditional.then_branch(),
                conditional.else_branch(),
            ]
            .into_iter()
            .flatten()
            {
                validate_expression(expression, scope, diagnostics);
            }
        }
        Expression::Assert(assertion) => {
            validate_pair(assertion.condition(), assertion.body(), scope, diagnostics);
        }
        Expression::With(with) => {
            if let Some(scope_expression) = with.scope() {
                validate_expression(scope_expression, scope, diagnostics);
            }
            scope.dynamic += 1;
            if let Some(body) = with.body() {
                validate_expression(body, scope, diagnostics);
            }
            scope.dynamic -= 1;
        }
        Expression::Lambda(lambda) => validate_function(lambda, scope, diagnostics),
        Expression::Apply(apply) => {
            validate_pair(apply.function(), apply.argument(), scope, diagnostics);
        }
        Expression::Select(select) => validate_select(select, scope, diagnostics),
        Expression::HasAttribute(test) => validate_has_attribute(test, scope, diagnostics),
        Expression::Unary(operation) => {
            if let Some(operand) = operation.operand() {
                validate_expression(operand, scope, diagnostics);
            }
        }
        Expression::Binary(operation) => {
            validate_binary(operation, scope, diagnostics);
        }
        Expression::Error(_) => {}
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
            }
            AttributeComponent::Interpolation(interpolation) => {
                if let Some(expression) = interpolation.expression() {
                    validate_expression(expression, scope, diagnostics);
                }
            }
            AttributeComponent::Identifier(_) => {}
        }
    }
}

fn validate_function<'doc, 'src: 'doc>(
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
    if let Some(body) = lambda.body() {
        validate_expression(body, scope, diagnostics);
    }
    scope.frames.pop();
}

fn validate_let<'doc, 'src: 'doc>(
    let_in: LetIn<'doc, 'src>,
    scope: &mut Scope<'src, '_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let frame = collect_entry_names(let_in.entries());
    scope.frames.push(frame);
    for entry in let_in.entries() {
        validate_entry(entry, scope, diagnostics);
    }
    if let Some(body) = let_in.body() {
        validate_expression(body, scope, diagnostics);
    }
    scope.frames.pop();
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
        }
        AttributeEntry::Inherit(inherit) => {
            let source = inherit.source();
            if let Some(source) = source {
                validate_expression(source, scope, diagnostics);
            } else {
                for name in inherit.identifiers() {
                    if !scope.contains(name.text()) {
                        diagnostics.push(Diagnostic::validation(
                            DiagnosticKind::UndefinedVariable,
                            Severity::Error,
                            name.range(),
                            name.kind(),
                        ));
                    }
                }
            }
        }
    }
}

fn collect_entry_names<'doc, 'src: 'doc>(
    entries: impl Iterator<Item = AttributeEntry<'doc, 'src>>,
) -> Vec<Cow<'src, str>> {
    let mut names = Vec::new();
    for entry in entries {
        match entry {
            AttributeEntry::Binding(binding) => {
                if let Some(path) = binding.path()
                    && let Some(name) = first_static_component(path)
                {
                    names.push(name);
                }
            }
            AttributeEntry::Inherit(inherit) => {
                names.extend(inherit.identifiers().map(|name| Cow::Borrowed(name.text())))
            }
        }
    }
    names
}

fn first_static_component<'doc, 'src: 'doc>(
    path: AttributePath<'doc, 'src>,
) -> Option<Cow<'src, str>> {
    match path.components().next()? {
        AttributeComponent::Identifier(token) => Some(Cow::Borrowed(token.text())),
        AttributeComponent::String(string) => static_string(string.syntax().text()).map(Cow::Owned),
        AttributeComponent::Interpolation(_) => None,
    }
}

const BASE_GLOBALS: &[&str] = &[
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
    "__floor",
    "__foldl",
    "__forceLazyFetcherAttr",
    "__fromJSON",
    "__functionArgs",
    "__genList",
    "__genericClosure",
    "__getAttr",
    "__getContext",
    "__getEnv",
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
    "fetchFinalTree",
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
    use crate::{DiagnosticKind, ParseOptions, UrlLiteralPolicy, parse, parse_with_options};

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
