use core::{
  fmt,
  iter::FusedIterator,
};

use crate::{
  Diagnostic,
  Severity,
  SyntaxKind,
  TextRange,
  TextSize,
};

const NONE: u32 = u32::MAX;
const COMPACT_NONE: u32 = 0x00FF_FFFF;

/// An element identifier within one parsed [`Document`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ElementId(u32);

impl ElementId {
  /// Creates an identifier from its compact numeric representation.
  #[must_use]
  pub const fn new(value: u32) -> Self {
    Self(value)
  }

  /// Returns the compact numeric representation.
  #[must_use]
  pub const fn get(self) -> u32 {
    self.0
  }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawElement {
  pub(crate) kind: SyntaxKind,
  range:           [u8; 8],
  parent:          [u8; 3],
  next_sibling:    [u8; 3],
}

const _: () = assert!(size_of::<RawElement>() == 15);

impl RawElement {
  pub(crate) fn new(kind: SyntaxKind, range: TextRange, parent: u32) -> Self {
    Self {
      kind,
      range: encode_range(range),
      parent: encode_link(parent),
      next_sibling: encode_link(NONE),
    }
  }

  fn range(self) -> TextRange {
    decode_range(self.range)
  }

  fn set_range(&mut self, range: TextRange) {
    self.range = encode_range(range);
  }

  fn parent(self) -> u32 {
    decode_link(self.parent)
  }

  fn next_sibling(self) -> u32 {
    decode_link(self.next_sibling)
  }

  fn set_next_sibling(&mut self, sibling: u32) {
    self.next_sibling = encode_link(sibling);
  }
}

fn encode_link(link: u32) -> [u8; 3] {
  let bytes = if link == NONE { COMPACT_NONE } else { link }.to_le_bytes();
  [bytes[0], bytes[1], bytes[2]]
}

fn decode_link(bytes: [u8; 3]) -> u32 {
  let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
  if value == COMPACT_NONE { NONE } else { value }
}

fn encode_range(range: TextRange) -> [u8; 8] {
  let start = range.start().get().to_le_bytes();
  let end = range.end().get().to_le_bytes();
  [
    start[0], start[1], start[2], start[3], end[0], end[1], end[2], end[3],
  ]
}

fn decode_range(bytes: [u8; 8]) -> TextRange {
  TextRange::new(
    TextSize::new(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
    TextSize::new(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])),
  )
}

const CHUNK_SHIFT: usize = 12;
const CHUNK_LEN: usize = 1 << CHUNK_SHIFT;
const CHUNK_MASK: usize = CHUNK_LEN - 1;

pub(crate) struct ElementStore {
  chunks: Box<[Box<[RawElement]>]>,
  len:    usize,
}

impl ElementStore {
  fn get(&self, index: usize) -> Option<&RawElement> {
    if index >= self.len {
      return None;
    }
    self
      .chunks
      .get(index >> CHUNK_SHIFT)?
      .get(index & CHUNK_MASK)
  }
}

struct ElementBuilder {
  chunks:  Vec<Box<[RawElement]>>,
  current: Vec<RawElement>,
  len:     usize,
}

impl ElementBuilder {
  fn new() -> Self {
    Self {
      chunks:  Vec::new(),
      current: Vec::with_capacity(CHUNK_LEN),
      len:     0,
    }
  }

  fn len(&self) -> usize {
    self.len
  }

  fn push(&mut self, element: RawElement) {
    if self.current.len() == CHUNK_LEN {
      let full =
        core::mem::replace(&mut self.current, Vec::with_capacity(CHUNK_LEN));
      self.chunks.push(full.into_boxed_slice());
    }
    self.current.push(element);
    self.len += 1;
  }

  fn get(&self, index: usize) -> &RawElement {
    let chunk = index >> CHUNK_SHIFT;
    if chunk < self.chunks.len() {
      &self.chunks[chunk][index & CHUNK_MASK]
    } else {
      &self.current[index & CHUNK_MASK]
    }
  }

  fn get_mut(&mut self, index: usize) -> &mut RawElement {
    let chunk = index >> CHUNK_SHIFT;
    if chunk < self.chunks.len() {
      &mut self.chunks[chunk][index & CHUNK_MASK]
    } else {
      &mut self.current[index & CHUNK_MASK]
    }
  }

  fn finish(mut self) -> ElementStore {
    if !self.current.is_empty() {
      self.chunks.push(self.current.into_boxed_slice());
    }
    ElementStore {
      chunks: self.chunks.into_boxed_slice(),
      len:    self.len,
    }
  }
}

/// A parsed Nix source file and its compact lossless syntax tree.
pub struct Document<'src> {
  pub(crate) source:      &'src str,
  pub(crate) elements:    ElementStore,
  pub(crate) diagnostics: Box<[Diagnostic]>,
}

impl fmt::Debug for Document<'_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("Document")
      .field("source_len", &self.source.len())
      .field("elements", &self.elements.len)
      .field("diagnostics", &self.diagnostics.len())
      .finish()
  }
}

impl<'src> Document<'src> {
  pub(crate) fn new(
    source: &'src str,
    elements: ElementStore,
    diagnostics: Vec<Diagnostic>,
  ) -> Self {
    Self {
      source,
      elements,
      diagnostics: diagnostics.into_boxed_slice(),
    }
  }

  /// Returns the original source.
  #[must_use]
  pub const fn source(&self) -> &'src str {
    self.source
  }

  /// Returns the root syntax node.
  #[must_use]
  pub fn root(&self) -> Node<'_, 'src> {
    Node {
      document: self,
      id:       ElementId::new(0),
    }
  }

  /// Looks up an element by its document-local identifier.
  #[must_use]
  pub fn element(&self, id: ElementId) -> Option<Element<'_, 'src>> {
    let raw = self.elements.get(id.0 as usize)?;
    Some(if raw.kind.is_token() {
      Element::Token(TokenNode { document: self, id })
    } else {
      Element::Node(Node { document: self, id })
    })
  }

  /// Returns the number of nodes and tokens in the document.
  #[must_use]
  pub fn element_count(&self) -> usize {
    self.elements.len
  }

  /// Returns all syntax and validation diagnostics.
  #[must_use]
  pub fn diagnostics(&self) -> &[Diagnostic] {
    &self.diagnostics
  }

  /// Returns whether parsing and validation found no errors.
  #[must_use]
  pub fn is_valid(&self) -> bool {
    self
      .diagnostics
      .iter()
      .all(|diagnostic| diagnostic.severity() != Severity::Error)
  }

  pub(crate) fn raw(&self, id: ElementId) -> &RawElement {
    self
      .elements
      .get(id.0 as usize)
      .expect("element identifier belongs to this document")
  }

  pub(crate) fn push_diagnostics(&mut self, diagnostics: Vec<Diagnostic>) {
    if diagnostics.is_empty() {
      return;
    }
    let mut combined =
      Vec::with_capacity(self.diagnostics.len() + diagnostics.len());
    combined.extend_from_slice(&self.diagnostics);
    combined.extend(diagnostics);
    self.diagnostics = combined.into_boxed_slice();
  }
}

/// A node or token in a parsed document.
#[derive(Clone, Copy, Debug)]
pub enum Element<'doc, 'src> {
  /// A grammatical node.
  Node(Node<'doc, 'src>),
  /// A lexical token.
  Token(TokenNode<'doc, 'src>),
}

impl<'doc, 'src> Element<'doc, 'src> {
  /// Returns the element identifier.
  #[must_use]
  pub const fn id(self) -> ElementId {
    match self {
      Self::Node(node) => node.id,
      Self::Token(token) => token.id,
    }
  }

  /// Returns the element kind.
  #[must_use]
  pub fn kind(self) -> SyntaxKind {
    match self {
      Self::Node(node) => node.kind(),
      Self::Token(token) => token.kind(),
    }
  }

  /// Returns the element's byte range.
  #[must_use]
  pub fn range(self) -> TextRange {
    match self {
      Self::Node(node) => node.range(),
      Self::Token(token) => token.range(),
    }
  }

  /// Returns the original text covered by the element.
  #[must_use]
  pub fn text(self) -> &'src str {
    match self {
      Self::Node(node) => node.text(),
      Self::Token(token) => token.text(),
    }
  }

  /// Returns the parent node, or `None` for the root.
  #[must_use]
  pub fn parent(self) -> Option<Node<'doc, 'src>> {
    match self {
      Self::Node(node) => node.parent(),
      Self::Token(token) => Some(token.parent()),
    }
  }
}

/// A grammatical node in a parsed document.
#[derive(Clone, Copy)]
pub struct Node<'doc, 'src> {
  document: &'doc Document<'src>,
  id:       ElementId,
}

impl fmt::Debug for Node<'_, '_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("Node")
      .field("id", &self.id)
      .field("kind", &self.kind())
      .field("range", &self.range())
      .finish()
  }
}

impl<'doc, 'src> Node<'doc, 'src> {
  /// Returns the node identifier.
  #[must_use]
  pub const fn id(self) -> ElementId {
    self.id
  }

  /// Returns the node kind.
  #[must_use]
  pub fn kind(self) -> SyntaxKind {
    self.document.raw(self.id).kind
  }

  /// Returns the node's byte range.
  #[must_use]
  pub fn range(self) -> TextRange {
    self.document.raw(self.id).range()
  }

  /// Returns the original text covered by the node.
  #[must_use]
  pub fn text(self) -> &'src str {
    let range = self.range();
    &self.document.source[range.start().as_usize()..range.end().as_usize()]
  }

  /// Returns the parent node, or `None` for the root.
  #[must_use]
  pub fn parent(self) -> Option<Self> {
    let parent = self.document.raw(self.id).parent();
    (parent != NONE).then_some(Self {
      document: self.document,
      id:       ElementId::new(parent),
    })
  }

  /// Iterates over immediate child nodes and tokens.
  #[must_use]
  pub fn children(self) -> Children<'doc, 'src> {
    let candidate = self.id.get().saturating_add(1);
    let next = self
      .document
      .elements
      .get(candidate as usize)
      .filter(|element| element.parent() == self.id.get())
      .map_or(NONE, |_| candidate);
    Children {
      document: self.document,
      next,
    }
  }

  /// Iterates over immediate child nodes.
  pub fn child_nodes(self) -> impl Iterator<Item = Self> {
    self.children().filter_map(|element| {
      match element {
        Element::Node(node) => Some(node),
        Element::Token(_) => None,
      }
    })
  }

  /// Returns the first immediate child node with `kind`.
  #[must_use]
  pub fn child(self, kind: SyntaxKind) -> Option<Self> {
    self.child_nodes().find(|node| node.kind() == kind)
  }

  /// Returns the first immediate token with `kind`.
  #[must_use]
  pub fn token(self, kind: SyntaxKind) -> Option<TokenNode<'doc, 'src>> {
    self.children().find_map(|element| {
      match element {
        Element::Token(token) if token.kind() == kind => Some(token),
        Element::Node(_) | Element::Token(_) => None,
      }
    })
  }
}

/// A lexical token in a parsed document.
#[derive(Clone, Copy)]
pub struct TokenNode<'doc, 'src> {
  document: &'doc Document<'src>,
  id:       ElementId,
}

impl fmt::Debug for TokenNode<'_, '_> {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    formatter
      .debug_struct("Token")
      .field("id", &self.id)
      .field("kind", &self.kind())
      .field("range", &self.range())
      .finish()
  }
}

impl<'doc, 'src> TokenNode<'doc, 'src> {
  /// Returns the token identifier.
  #[must_use]
  pub const fn id(self) -> ElementId {
    self.id
  }

  /// Returns the token kind.
  #[must_use]
  pub fn kind(self) -> SyntaxKind {
    self.document.raw(self.id).kind
  }

  /// Returns the token's byte range.
  #[must_use]
  pub fn range(self) -> TextRange {
    self.document.raw(self.id).range()
  }

  /// Returns the token's original source text.
  #[must_use]
  pub fn text(self) -> &'src str {
    let range = self.range();
    &self.document.source[range.start().as_usize()..range.end().as_usize()]
  }

  /// Returns the token's parent node.
  #[must_use]
  pub fn parent(self) -> Node<'doc, 'src> {
    Node {
      document: self.document,
      id:       ElementId::new(self.document.raw(self.id).parent()),
    }
  }
}

/// An iterator over the immediate children of a syntax node.
#[derive(Clone, Debug)]
pub struct Children<'doc, 'src> {
  document: &'doc Document<'src>,
  next:     u32,
}

impl<'doc, 'src> Iterator for Children<'doc, 'src> {
  type Item = Element<'doc, 'src>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.next == NONE {
      return None;
    }
    let id = ElementId::new(self.next);
    let raw = self.document.raw(id);
    self.next = raw.next_sibling();
    self.document.element(id)
  }
}

impl FusedIterator for Children<'_, '_> {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum EventTag {
  Start,
  Token,
  Finish,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Event {
  tag:  EventTag,
  kind: SyntaxKind,
  data: [u8; 8],
}

const _: () = assert!(size_of::<Event>() == 10);

impl Event {
  pub(crate) const fn start() -> Self {
    Self {
      tag:  EventTag::Start,
      kind: SyntaxKind::Eof,
      data: [u8::MAX; 8],
    }
  }

  pub(crate) fn token(token: crate::Token) -> Self {
    Self {
      tag:  EventTag::Token,
      kind: token.kind(),
      data: encode_range(token.range()),
    }
  }

  pub(crate) const fn finish() -> Self {
    Self {
      tag:  EventTag::Finish,
      kind: SyntaxKind::Eof,
      data: [0; 8],
    }
  }

  fn start_kind(self) -> Option<SyntaxKind> {
    (self.tag == EventTag::Start && self.kind != SyntaxKind::Eof)
      .then_some(self.kind)
  }

  pub(crate) fn set_start_kind(&mut self, kind: SyntaxKind) {
    assert!(
      self.tag == EventTag::Start,
      "marker did not point to a start event"
    );
    self.kind = kind;
  }

  fn take_start_kind(&mut self) -> Option<SyntaxKind> {
    let kind = self.start_kind()?;
    self.kind = SyntaxKind::Eof;
    Some(kind)
  }

  fn forward_parent(self) -> Option<usize> {
    if self.tag != EventTag::Start {
      return None;
    }
    let value =
      u32::from_le_bytes(self.data[..4].try_into().expect("four-byte slice"));
    (value != u32::MAX).then_some(value as usize)
  }

  pub(crate) fn set_forward_parent(&mut self, distance: usize) {
    assert!(
      self.tag == EventTag::Start,
      "marker did not point to a start event"
    );
    let distance = u32::try_from(distance)
      .expect("event distance fits compact source offsets");
    self.data[..4].copy_from_slice(&distance.to_le_bytes());
  }

  fn token_value(self) -> Option<crate::Token> {
    (self.tag == EventTag::Token)
      .then(|| crate::Token::new(self.kind, decode_range(self.data)))
  }
}

pub(crate) struct EventBuffer {
  chunks:  Vec<Option<Box<[Event]>>>,
  current: Vec<Event>,
  len:     usize,
}

impl EventBuffer {
  pub(crate) fn with_source_capacity(source_len: usize) -> Self {
    Self {
      chunks:  Vec::with_capacity(source_len / (CHUNK_LEN * 2)),
      current: Vec::with_capacity(CHUNK_LEN),
      len:     0,
    }
  }

  pub(crate) const fn len(&self) -> usize {
    self.len
  }

  pub(crate) fn push(&mut self, event: Event) {
    if self.current.len() == CHUNK_LEN {
      let full =
        core::mem::replace(&mut self.current, Vec::with_capacity(CHUNK_LEN));
      self.chunks.push(Some(full.into_boxed_slice()));
    }
    self.current.push(event);
    self.len += 1;
  }

  pub(crate) fn get_mut(&mut self, index: usize) -> &mut Event {
    let chunk = index >> CHUNK_SHIFT;
    if chunk < self.chunks.len() {
      &mut self.chunks[chunk]
        .as_mut()
        .expect("event chunk is still live")[index & CHUNK_MASK]
    } else {
      &mut self.current[index & CHUNK_MASK]
    }
  }

  fn seal(&mut self) {
    if !self.current.is_empty() {
      let current = core::mem::take(&mut self.current);
      self.chunks.push(Some(current.into_boxed_slice()));
    }
  }

  fn get(&self, index: usize) -> Event {
    self.chunks[index >> CHUNK_SHIFT]
      .as_ref()
      .expect("event chunk is still live")[index & CHUNK_MASK]
  }

  fn release_through(&mut self, index: usize) {
    if index & CHUNK_MASK == CHUNK_MASK {
      self.chunks[index >> CHUNK_SHIFT] = None;
    }
  }
}

#[derive(Clone, Copy, Debug)]
struct Frame {
  element:    u32,
  last_child: u32,
}

pub(crate) fn build_tree(
  events: &mut EventBuffer,
) -> Result<ElementStore, crate::InputError> {
  events.seal();
  let mut elements = ElementBuilder::new();
  let mut stack: Vec<Frame> = Vec::with_capacity(32);
  let mut kinds = Vec::with_capacity(4);
  let mut last_end = TextSize::new(0);

  for index in 0..events.len() {
    let event = events.get(index);
    match event.tag {
      EventTag::Start if event.start_kind().is_none() => {},
      EventTag::Start => {
        kinds.clear();
        let mut cursor = index;
        loop {
          let event = events.get_mut(cursor);
          if event.tag != EventTag::Start {
            break;
          }
          if let Some(kind) = event.take_start_kind() {
            kinds.push(kind);
          }
          let Some(distance) = event.forward_parent() else {
            break;
          };
          cursor += distance;
        }
        for kind in kinds.drain(..).rev() {
          start_element(&mut elements, &mut stack, kind, last_end)?;
        }
      },
      EventTag::Token => {
        let token = event.token_value().expect("tag checked");
        if elements.len() == COMPACT_NONE as usize {
          return Err(crate::InputError::TooManyElements);
        }
        let id = elements.len() as u32;
        let parent = stack.last().map_or(NONE, |frame| frame.element);
        elements.push(RawElement::new(token.kind(), token.range(), parent));
        link_child(&mut elements, &mut stack, id);
        last_end = token.range().end();
      },
      EventTag::Finish => {
        let frame = stack
          .pop()
          .expect("parser emitted an unmatched finish event");
        let element = elements.get_mut(frame.element as usize);
        element.set_range(TextRange::new(element.range().start(), last_end));
      },
    }
    events.release_through(index);
  }
  debug_assert!(stack.is_empty());
  Ok(elements.finish())
}

fn start_element(
  elements: &mut ElementBuilder,
  stack: &mut Vec<Frame>,
  kind: SyntaxKind,
  position: TextSize,
) -> Result<(), crate::InputError> {
  if elements.len() == COMPACT_NONE as usize {
    return Err(crate::InputError::TooManyElements);
  }
  let id = elements.len() as u32;
  let parent = stack.last().map_or(NONE, |frame| frame.element);
  elements.push(RawElement::new(
    kind,
    TextRange::new(position, position),
    parent,
  ));
  link_child(elements, stack, id);
  stack.push(Frame {
    element:    id,
    last_child: NONE,
  });
  Ok(())
}

fn link_child(elements: &mut ElementBuilder, stack: &mut [Frame], child: u32) {
  let Some(parent) = stack.last_mut() else {
    return;
  };
  let child_start = elements.get(child as usize).range().start();
  let parent_element = elements.get_mut(parent.element as usize);
  if parent.last_child == NONE {
    parent_element
      .set_range(TextRange::new(child_start, parent_element.range().end()));
  } else {
    elements
      .get_mut(parent.last_child as usize)
      .set_next_sibling(child);
  }
  parent.last_child = child;
}
