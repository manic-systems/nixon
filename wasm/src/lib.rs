//! WebAssembly bindings for Nixon's indexed syntax tree.

use nixon::{
    Diagnostic, Element, ElementId, ParseOptions, SyntaxKind, UrlLiteralPolicy, parse_with_options,
};
use wasm_bindgen::prelude::*;

const NONE: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct ElementRecord {
    kind: SyntaxKind,
    start: u32,
    end: u32,
    parent: u32,
    first_child: u32,
    next_sibling: u32,
}

const _: () = assert!(size_of::<ElementRecord>() <= 24);

/// A parse result exposed to JavaScript as an indexed syntax tree.
#[wasm_bindgen(js_name = ParsedNix)]
pub struct ParsedNix {
    source: String,
    elements: Box<[ElementRecord]>,
    diagnostics: Box<[Diagnostic]>,
    valid: bool,
}

/// Parses Nix source with Nix 2.35's default syntax settings.
///
/// # Errors
///
/// Returns a JavaScript error if the source exceeds Nixon's input limit.
#[wasm_bindgen(js_name = parse)]
pub fn parse_js(source: String) -> Result<ParsedNix, JsError> {
    parse_owned(source, ParseOptions::default())
}

/// Parses Nix source with explicit feature switches.
///
/// # Errors
///
/// Returns a JavaScript error if the source exceeds Nixon's input limit.
#[wasm_bindgen(js_name = parseWithOptions)]
pub fn parse_with_options_js(
    source: String,
    pipe_operators: bool,
    deny_uri_literals: bool,
    validate_identifiers: bool,
) -> Result<ParsedNix, JsError> {
    parse_owned(
        source,
        ParseOptions {
            pipe_operators,
            uri_literals: if deny_uri_literals {
                UrlLiteralPolicy::Deny
            } else {
                UrlLiteralPolicy::Warn
            },
            validate_identifiers,
            ..ParseOptions::default()
        },
    )
}

fn parse_owned(source: String, options: ParseOptions<'_>) -> Result<ParsedNix, JsError> {
    let document =
        parse_with_options(&source, options).map_err(|error| JsError::new(&error.to_string()))?;
    let mut elements = Vec::with_capacity(document.element_count());
    for index in 0..document.element_count() {
        let element = document
            .element(ElementId::new(index as u32))
            .expect("element count bounds were checked");
        let range = element.range();
        elements.push(ElementRecord {
            kind: element.kind(),
            start: range.start().get(),
            end: range.end().get(),
            parent: element.parent().map_or(NONE, |parent| parent.id().get()),
            first_child: NONE,
            next_sibling: NONE,
        });
    }
    for index in 0..document.element_count() {
        let Some(Element::Node(node)) = document.element(ElementId::new(index as u32)) else {
            continue;
        };
        let mut previous = NONE;
        for child in node.children() {
            let child = child.id().get();
            if previous == NONE {
                elements[index].first_child = child;
            } else {
                elements[previous as usize].next_sibling = child;
            }
            previous = child;
        }
    }
    let diagnostics = document.diagnostics().to_vec().into_boxed_slice();
    let valid = document.is_valid();
    drop(document);
    Ok(ParsedNix {
        source,
        elements: elements.into_boxed_slice(),
        diagnostics,
        valid,
    })
}

#[wasm_bindgen(js_class = ParsedNix)]
impl ParsedNix {
    /// Returns the original UTF-8 source.
    #[wasm_bindgen(getter)]
    pub fn source(&self) -> String {
        self.source.clone()
    }

    /// Returns whether parsing and validation found no errors.
    #[wasm_bindgen(js_name = isValid)]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Returns the number of indexed syntax elements.
    #[wasm_bindgen(js_name = elementCount)]
    pub fn element_count(&self) -> u32 {
        self.elements.len() as u32
    }

    /// Returns the number of diagnostics.
    #[wasm_bindgen(js_name = diagnosticCount)]
    pub fn diagnostic_count(&self) -> u32 {
        self.diagnostics.len() as u32
    }

    /// Returns an element's compact kind discriminant.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    pub fn kind(&self, id: u32) -> Result<u8, JsError> {
        Ok(self.element(id)?.kind as u8)
    }

    /// Returns an element's stable Rust kind name.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    #[wasm_bindgen(js_name = kindName)]
    pub fn kind_name(&self, id: u32) -> Result<String, JsError> {
        Ok(format!("{:?}", self.element(id)?.kind))
    }

    /// Returns an element's first byte offset.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    #[wasm_bindgen(js_name = rangeStart)]
    pub fn range_start(&self, id: u32) -> Result<u32, JsError> {
        Ok(self.element(id)?.start)
    }

    /// Returns the byte offset immediately after an element.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    #[wasm_bindgen(js_name = rangeEnd)]
    pub fn range_end(&self, id: u32) -> Result<u32, JsError> {
        Ok(self.element(id)?.end)
    }

    /// Returns an element's parent identifier.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    pub fn parent(&self, id: u32) -> Result<Option<u32>, JsError> {
        Ok(as_option(self.element(id)?.parent))
    }

    /// Returns a node's first child identifier.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    #[wasm_bindgen(js_name = firstChild)]
    pub fn first_child(&self, id: u32) -> Result<Option<u32>, JsError> {
        Ok(as_option(self.element(id)?.first_child))
    }

    /// Returns an element's next sibling identifier.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid element identifier.
    #[wasm_bindgen(js_name = nextSibling)]
    pub fn next_sibling(&self, id: u32) -> Result<Option<u32>, JsError> {
        Ok(as_option(self.element(id)?.next_sibling))
    }

    /// Returns a diagnostic's message.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid diagnostic identifier.
    #[wasm_bindgen(js_name = diagnosticMessage)]
    pub fn diagnostic_message(&self, id: u32) -> Result<String, JsError> {
        Ok(self.diagnostic(id)?.to_string())
    }

    /// Returns a diagnostic's severity, where zero is an error and one is a warning.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid diagnostic identifier.
    #[wasm_bindgen(js_name = diagnosticSeverity)]
    pub fn diagnostic_severity(&self, id: u32) -> Result<u8, JsError> {
        Ok(self.diagnostic(id)?.severity() as u8)
    }

    /// Returns a diagnostic's first byte offset.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid diagnostic identifier.
    #[wasm_bindgen(js_name = diagnosticStart)]
    pub fn diagnostic_start(&self, id: u32) -> Result<u32, JsError> {
        Ok(self.diagnostic(id)?.range().start().get())
    }

    /// Returns the byte offset immediately after a diagnostic.
    ///
    /// # Errors
    ///
    /// Returns a JavaScript error for an invalid diagnostic identifier.
    #[wasm_bindgen(js_name = diagnosticEnd)]
    pub fn diagnostic_end(&self, id: u32) -> Result<u32, JsError> {
        Ok(self.diagnostic(id)?.range().end().get())
    }

    fn element(&self, id: u32) -> Result<&ElementRecord, JsError> {
        self.elements
            .get(id as usize)
            .ok_or_else(|| JsError::new("invalid syntax element identifier"))
    }

    fn diagnostic(&self, id: u32) -> Result<&Diagnostic, JsError> {
        self.diagnostics
            .get(id as usize)
            .ok_or_else(|| JsError::new("invalid diagnostic identifier"))
    }
}

fn as_option(value: u32) -> Option<u32> {
    (value != NONE).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_owned_indexed_tree() {
        let options = ParseOptions {
            validate_identifiers: false,
            ..ParseOptions::default()
        };
        let parsed = parse_owned("{ x = [ 1 2 ]; }".to_owned(), options).expect("small source");
        assert!(parsed.is_valid());
        assert!(parsed.element_count() > 1);
        assert_eq!(parsed.parent(0).expect("root exists"), None);
        assert!(parsed.first_child(0).expect("root exists").is_some());
        for id in 1..parsed.element_count() {
            assert!(parsed.parent(id).expect("element exists").is_some());
            assert!(
                parsed.range_start(id).expect("element exists")
                    <= parsed.range_end(id).expect("element exists")
            );
        }
    }
}
