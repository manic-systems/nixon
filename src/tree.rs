use core::{fmt, iter::FusedIterator};

use crate::{Diagnostic, Severity, SyntaxKind, TextRange, TextSize};

const NONE: u32 = u32::MAX;

/// An element identifier within one parsed [`Document`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ElementId(u32);

impl ElementId {
    pub(crate) const fn new(value: u32) -> Self {
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
    kind: SyntaxKind,
    range: TextRange,
    parent: u32,
    first_child: u32,
    next_sibling: u32,
}

const _: () = assert!(size_of::<RawElement>() <= 24);

impl RawElement {
    pub(crate) const fn new(kind: SyntaxKind, range: TextRange, parent: u32) -> Self {
        Self {
            kind,
            range,
            parent,
            first_child: NONE,
            next_sibling: NONE,
        }
    }
}

/// A parsed Nix source file and its compact lossless syntax tree.
pub struct Document<'src> {
    pub(crate) source: &'src str,
    pub(crate) elements: Box<[RawElement]>,
    pub(crate) diagnostics: Box<[Diagnostic]>,
}

impl fmt::Debug for Document<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Document")
            .field("source_len", &self.source.len())
            .field("elements", &self.elements.len())
            .field("diagnostics", &self.diagnostics.len())
            .finish()
    }
}

impl<'src> Document<'src> {
    pub(crate) fn new(
        source: &'src str,
        elements: Vec<RawElement>,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            source,
            elements: elements.into_boxed_slice(),
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
            id: ElementId::new(0),
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
        self.elements.len()
    }

    /// Returns all syntax and validation diagnostics.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns whether parsing and validation found no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != Severity::Error)
    }

    fn raw(&self, id: ElementId) -> &RawElement {
        &self.elements[id.0 as usize]
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
}

/// A grammatical node in a parsed document.
#[derive(Clone, Copy)]
pub struct Node<'doc, 'src> {
    document: &'doc Document<'src>,
    id: ElementId,
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
        self.document.raw(self.id).range
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
        let parent = self.document.raw(self.id).parent;
        (parent != NONE).then_some(Self {
            document: self.document,
            id: ElementId::new(parent),
        })
    }

    /// Iterates over immediate child nodes and tokens.
    #[must_use]
    pub fn children(self) -> Children<'doc, 'src> {
        Children {
            document: self.document,
            next: self.document.raw(self.id).first_child,
        }
    }

    /// Iterates over immediate child nodes.
    pub fn child_nodes(self) -> impl Iterator<Item = Self> {
        self.children().filter_map(|element| match element {
            Element::Node(node) => Some(node),
            Element::Token(_) => None,
        })
    }
}

/// A lexical token in a parsed document.
#[derive(Clone, Copy)]
pub struct TokenNode<'doc, 'src> {
    document: &'doc Document<'src>,
    id: ElementId,
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
        self.document.raw(self.id).range
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
            id: ElementId::new(self.document.raw(self.id).parent),
        }
    }
}

/// An iterator over the immediate children of a syntax node.
#[derive(Clone, Debug)]
pub struct Children<'doc, 'src> {
    document: &'doc Document<'src>,
    next: u32,
}

impl<'doc, 'src> Iterator for Children<'doc, 'src> {
    type Item = Element<'doc, 'src>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next == NONE {
            return None;
        }
        let id = ElementId::new(self.next);
        let raw = self.document.raw(id);
        self.next = raw.next_sibling;
        self.document.element(id)
    }
}

impl FusedIterator for Children<'_, '_> {}

#[derive(Debug)]
pub(crate) enum Event {
    Start {
        kind: Option<SyntaxKind>,
        forward_parent: Option<usize>,
    },
    Token(crate::Token),
    Finish,
}

#[derive(Clone, Copy, Debug)]
struct Frame {
    element: u32,
    last_child: u32,
}

pub(crate) fn build_tree(events: &mut [Event]) -> Vec<RawElement> {
    let mut elements = Vec::with_capacity(events.len() / 2);
    let mut stack: Vec<Frame> = Vec::new();
    let mut last_end = TextSize::new(0);

    for index in 0..events.len() {
        match &events[index] {
            Event::Start { kind: None, .. } => {}
            Event::Start { kind: Some(_), .. } => {
                let mut kinds = Vec::new();
                let mut cursor = index;
                while let Event::Start {
                    kind,
                    forward_parent,
                } = &mut events[cursor]
                {
                    if let Some(kind) = kind.take() {
                        kinds.push(kind);
                    }
                    let Some(distance) = forward_parent.take() else {
                        break;
                    };
                    cursor += distance;
                }
                for kind in kinds.into_iter().rev() {
                    start_element(&mut elements, &mut stack, kind, last_end);
                }
            }
            Event::Token(token) => {
                let id = elements.len() as u32;
                let parent = stack.last().map_or(NONE, |frame| frame.element);
                elements.push(RawElement::new(token.kind(), token.range(), parent));
                link_child(&mut elements, &mut stack, id);
                last_end = token.range().end();
            }
            Event::Finish => {
                let frame = stack
                    .pop()
                    .expect("parser emitted an unmatched finish event");
                let element = &mut elements[frame.element as usize];
                element.range = TextRange::new(element.range.start(), last_end);
            }
        }
    }
    debug_assert!(stack.is_empty());
    elements
}

fn start_element(
    elements: &mut Vec<RawElement>,
    stack: &mut Vec<Frame>,
    kind: SyntaxKind,
    position: TextSize,
) {
    let id = elements.len() as u32;
    let parent = stack.last().map_or(NONE, |frame| frame.element);
    elements.push(RawElement::new(
        kind,
        TextRange::new(position, position),
        parent,
    ));
    link_child(elements, stack, id);
    stack.push(Frame {
        element: id,
        last_child: NONE,
    });
}

fn link_child(elements: &mut [RawElement], stack: &mut [Frame], child: u32) {
    let Some(parent) = stack.last_mut() else {
        return;
    };
    let child_start = elements[child as usize].range.start();
    let parent_element = &mut elements[parent.element as usize];
    if parent_element.first_child == NONE {
        parent_element.first_child = child;
        parent_element.range = TextRange::new(child_start, parent_element.range.end());
    } else {
        elements[parent.last_child as usize].next_sibling = child;
    }
    parent.last_child = child;
}
