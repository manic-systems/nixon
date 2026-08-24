//! C bindings for Nixon's indexed syntax tree.

use core::{
  ptr,
  slice,
};

use nixon::{
  DiagnosticKind,
  Element,
  ElementId,
  InputError,
  parse_bytes,
};

/// No related element exists.
pub const NIXON_ELEMENT_NONE: u32 = u32::MAX;

/// A successful operation.
pub const NIXON_STATUS_OK: u8 = 0;
/// A required pointer was null.
pub const NIXON_STATUS_INVALID_ARGUMENT: u8 = 1;
/// The source was not valid UTF-8.
pub const NIXON_STATUS_INVALID_UTF8: u8 = 2;
/// The source exceeded Nixon's input limit.
pub const NIXON_STATUS_TOO_LARGE: u8 = 3;
/// The tree exceeded Nixon's element limit.
pub const NIXON_STATUS_TOO_MANY_ELEMENTS: u8 = 4;
/// A newer parser rejected the input for an unrecognized reason.
pub const NIXON_STATUS_UNKNOWN_INPUT_ERROR: u8 = 5;

/// A borrowed byte string owned by a [`NixonDocument`].
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NixonString {
  /// The first byte, which is not null-terminated.
  pub data: *const u8,
  /// The number of bytes.
  pub len:  usize,
}

/// One node or token in an indexed syntax tree.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NixonElement {
  /// The `NIXON_SYNTAX_*` kind value.
  pub kind:         u8,
  /// The first UTF-8 byte offset.
  pub start:        u32,
  /// The byte offset immediately after the element.
  pub end:          u32,
  /// The parent identifier, or [`NIXON_ELEMENT_NONE`].
  pub parent:       u32,
  /// The first child identifier, or [`NIXON_ELEMENT_NONE`].
  pub first_child:  u32,
  /// The next sibling identifier, or [`NIXON_ELEMENT_NONE`].
  pub next_sibling: u32,
}

struct OwnedDiagnostic {
  kind:     u8,
  severity: u8,
  start:    u32,
  end:      u32,
  message:  Box<str>,
}

/// One syntax or validation diagnostic.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NixonDiagnostic {
  /// The `NIXON_DIAGNOSTIC_*` kind value.
  pub kind:     u8,
  /// Zero for an error, one for a warning.
  pub severity: u8,
  /// The first UTF-8 byte offset.
  pub start:    u32,
  /// The byte offset immediately after the diagnostic.
  pub end:      u32,
  /// A UTF-8 message borrowed from the document.
  pub message:  NixonString,
}

/// An owned parse result.
pub struct NixonDocument {
  elements:    Box<[NixonElement]>,
  diagnostics: Box<[OwnedDiagnostic]>,
  valid:       bool,
}

/// Parses UTF-8 Nix source with Nixon's default settings.
///
/// The parser borrows `source` only for this call. On success, `*document`
/// receives an owned handle that must be passed to [`nixon_document_free`].
/// Syntax errors are diagnostics and do not make this function fail.
///
/// # Safety
///
/// `document` must be valid for writes. When `source_len` is nonzero, `source`
/// must be valid for reads of that many bytes. The input memory must not change
/// during this call.
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_parse(
  source: *const u8,
  source_len: usize,
  document: *mut *mut NixonDocument,
) -> u8 {
  if document.is_null() {
    return NIXON_STATUS_INVALID_ARGUMENT;
  }
  // SAFETY: the caller guarantees that document is valid for writes.
  unsafe { document.write(ptr::null_mut()) };
  if source_len > u32::MAX as usize {
    return NIXON_STATUS_TOO_LARGE;
  }
  if source_len != 0 && source.is_null() {
    return NIXON_STATUS_INVALID_ARGUMENT;
  }
  let bytes = if source_len == 0 {
    &[]
  } else {
    // SAFETY: the caller guarantees a readable, stable source allocation.
    unsafe { slice::from_raw_parts(source, source_len) }
  };
  let parsed = match Parsed::new(bytes) {
    Ok(parsed) => parsed,
    Err(status) => return status,
  };
  // SAFETY: document is non-null and valid for writes by the caller's contract.
  unsafe { document.write(Box::into_raw(Box::new(parsed.0))) };
  NIXON_STATUS_OK
}

struct Parsed(NixonDocument);

impl Parsed {
  fn new(source: &[u8]) -> Result<Self, u8> {
    let document = parse_bytes(source).map_err(input_error_status)?;
    let mut elements = Vec::with_capacity(document.element_count());
    for index in 0..document.element_count() {
      let element = document
        .element(ElementId::new(index as u32))
        .expect("element count bounds were checked");
      let range = element.range();
      elements.push(NixonElement {
        kind:         element.kind() as u8,
        start:        range.start().get(),
        end:          range.end().get(),
        parent:       element
          .parent()
          .map_or(NIXON_ELEMENT_NONE, |parent| parent.id().get()),
        first_child:  NIXON_ELEMENT_NONE,
        next_sibling: NIXON_ELEMENT_NONE,
      });
    }
    for index in 0..document.element_count() {
      let Some(Element::Node(node)) =
        document.element(ElementId::new(index as u32))
      else {
        continue;
      };
      let mut previous = NIXON_ELEMENT_NONE;
      for child in node.children() {
        let child = child.id().get();
        if previous == NIXON_ELEMENT_NONE {
          elements[index].first_child = child;
        } else {
          elements[previous as usize].next_sibling = child;
        }
        previous = child;
      }
    }
    let diagnostics = document
      .diagnostics()
      .iter()
      .map(|diagnostic| {
        let range = diagnostic.range();
        OwnedDiagnostic {
          kind:     diagnostic_kind(diagnostic.kind()),
          severity: diagnostic.severity() as u8,
          start:    range.start().get(),
          end:      range.end().get(),
          message:  diagnostic.to_string().into_boxed_str(),
        }
      })
      .collect();
    Ok(Self(NixonDocument {
      elements: elements.into_boxed_slice(),
      diagnostics,
      valid: document.is_valid(),
    }))
  }
}

fn input_error_status(error: InputError) -> u8 {
  match error {
    InputError::InvalidUtf8 { .. } => NIXON_STATUS_INVALID_UTF8,
    InputError::TooLarge => NIXON_STATUS_TOO_LARGE,
    InputError::TooManyElements => NIXON_STATUS_TOO_MANY_ELEMENTS,
    _ => NIXON_STATUS_UNKNOWN_INPUT_ERROR,
  }
}

fn diagnostic_kind(kind: DiagnosticKind) -> u8 {
  match kind {
    DiagnosticKind::UnexpectedToken => 0,
    DiagnosticKind::UnexpectedEof => 1,
    DiagnosticKind::InvalidToken => 2,
    DiagnosticKind::TrailingInput => 3,
    DiagnosticKind::NestingLimit => 4,
    DiagnosticKind::ExperimentalFeatureDisabled => 5,
    DiagnosticKind::UriLiteral => 6,
    DiagnosticKind::IntegerOverflow => 7,
    DiagnosticKind::FloatOutOfRange => 8,
    DiagnosticKind::TrailingSlashPath => 9,
    DiagnosticKind::DuplicateFormal => 10,
    DiagnosticKind::DuplicateAttribute => 11,
    DiagnosticKind::ConflictingAttribute => 12,
    DiagnosticKind::DynamicAttributeInLet => 13,
    DiagnosticKind::DynamicAttributeInInherit => 14,
    DiagnosticKind::UndefinedVariable => 15,
    _ => u8::MAX,
  }
}

/// Frees an owned parse result. A null pointer is accepted.
///
/// # Safety
///
/// `document` must be null or a live handle returned by [`nixon_parse`] that
/// has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_free(document: *mut NixonDocument) {
  if !document.is_null() {
    // SAFETY: the caller transfers the unique live allocation back to Rust.
    drop(unsafe { Box::from_raw(document) });
  }
}

/// Returns one when a document has no error diagnostics.
///
/// # Safety
///
/// `document` must point to a live [`NixonDocument`].
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_is_valid(
  document: *const NixonDocument,
) -> u8 {
  if document.is_null() {
    return 0;
  }
  // SAFETY: the caller guarantees a live document for the duration of the call.
  unsafe { (*document).valid.into() }
}

/// Returns the number of indexed syntax elements.
///
/// # Safety
///
/// `document` must point to a live [`NixonDocument`].
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_element_count(
  document: *const NixonDocument,
) -> usize {
  if document.is_null() {
    return 0;
  }
  // SAFETY: the caller guarantees a live document for the duration of the call.
  unsafe { (&*document).elements.len() }
}

/// Copies one indexed element into `element`, returning one on success.
///
/// # Safety
///
/// `document` must point to a live [`NixonDocument`], and `element` must be
/// valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_element(
  document: *const NixonDocument,
  id: u32,
  element: *mut NixonElement,
) -> u8 {
  if document.is_null() || element.is_null() {
    return 0;
  }
  // SAFETY: the caller guarantees a live document for the duration of the call.
  let Some(value) = (unsafe { &*document }).elements.get(id as usize) else {
    return 0;
  };
  // SAFETY: the caller guarantees element is valid for writes.
  unsafe { element.write(*value) };
  1
}

/// Returns the number of diagnostics.
///
/// # Safety
///
/// `document` must point to a live [`NixonDocument`].
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_diagnostic_count(
  document: *const NixonDocument,
) -> usize {
  if document.is_null() {
    return 0;
  }
  // SAFETY: the caller guarantees a live document for the duration of the call.
  unsafe { (&*document).diagnostics.len() }
}

/// Copies one diagnostic into `diagnostic`, returning one on success.
///
/// The message remains valid until the document is freed.
///
/// # Safety
///
/// `document` must point to a live [`NixonDocument`], and `diagnostic` must be
/// valid for writes.
#[unsafe(no_mangle)]
pub unsafe extern fn nixon_document_diagnostic(
  document: *const NixonDocument,
  id: usize,
  diagnostic: *mut NixonDiagnostic,
) -> u8 {
  if document.is_null() || diagnostic.is_null() {
    return 0;
  }
  // SAFETY: the caller guarantees a live document for the duration of the call.
  let Some(value) = (unsafe { &*document }).diagnostics.get(id) else {
    return 0;
  };
  let message = value.message.as_bytes();
  // SAFETY: the caller guarantees diagnostic is valid for writes.
  unsafe {
    diagnostic.write(NixonDiagnostic {
      kind:     value.kind,
      severity: value.severity,
      start:    value.start,
      end:      value.end,
      message:  NixonString {
        data: message.as_ptr(),
        len:  message.len(),
      },
    })
  };
  1
}

#[cfg(test)]
mod tests {
  use nixon::SyntaxKind;

  use super::*;

  #[test]
  fn parses_and_owns_an_indexed_tree() {
    let source = b"{ answer = missing; }";
    let mut document = ptr::null_mut();
    // SAFETY: source and the output pointer remain valid for the call.
    let status =
      unsafe { nixon_parse(source.as_ptr(), source.len(), &mut document) };
    assert_eq!(status, NIXON_STATUS_OK);
    assert!(!document.is_null());

    // SAFETY: document is a live handle returned above.
    let count = unsafe { nixon_document_element_count(document) };
    let mut root = NixonElement {
      kind:         0,
      start:        0,
      end:          0,
      parent:       0,
      first_child:  0,
      next_sibling: 0,
    };
    // SAFETY: document is live and root is writable.
    assert_eq!(unsafe { nixon_document_element(document, 0, &mut root) }, 1);
    assert!(count > 1);
    assert_eq!(root.kind, SyntaxKind::Root as u8);
    assert_eq!(root.parent, NIXON_ELEMENT_NONE);
    assert_ne!(root.first_child, NIXON_ELEMENT_NONE);

    let mut diagnostic = NixonDiagnostic {
      kind:     0,
      severity: 0,
      start:    0,
      end:      0,
      message:  NixonString {
        data: ptr::null(),
        len:  0,
      },
    };
    // SAFETY: document is live and diagnostic is writable.
    assert_eq!(
      unsafe { nixon_document_diagnostic(document, 0, &mut diagnostic) },
      1
    );
    // SAFETY: the returned message is borrowed from the still-live document.
    let message = unsafe {
      slice::from_raw_parts(diagnostic.message.data, diagnostic.message.len)
    };
    assert_eq!(message, b"undefined variable");

    // SAFETY: this returns the unique live allocation exactly once.
    unsafe { nixon_document_free(document) };
  }

  #[test]
  fn rejects_invalid_boundary_arguments() {
    let mut document = ptr::null_mut();
    // SAFETY: the output pointer is writable; the null source is rejected.
    assert_eq!(
      unsafe { nixon_parse(ptr::null(), 1, &mut document) },
      NIXON_STATUS_INVALID_ARGUMENT
    );
    // SAFETY: the one-byte input and output pointer are valid.
    assert_eq!(
      unsafe { nixon_parse([0xFF].as_ptr(), 1, &mut document) },
      NIXON_STATUS_INVALID_UTF8
    );
  }
}
