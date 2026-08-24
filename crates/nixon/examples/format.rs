//! A deliberately small formatter built from Nixon's lossless token stream.

use std::{
  env,
  error::Error,
  fmt::Write as _,
  io::Write as _,
};

use nixon::{
  Document,
  Element,
  ElementId,
  SyntaxKind,
  parse_syntax,
};

const SAMPLE: &str =
  "{foo=1;# comments survive\nnested={list=[1 2 3];enabled=true;};}";

fn main() -> Result<(), Box<dyn Error>> {
  let mut arguments = env::args().skip(1);
  let first = arguments.next();
  let (dump, source) = if first.as_deref() == Some("--dump") {
    (
      true,
      arguments.next().unwrap_or_else(|| "[ phlegm  ]".to_owned()),
    )
  } else {
    (false, first.unwrap_or_else(|| SAMPLE.to_owned()))
  };
  let document = parse_syntax(&source)?;
  for diagnostic in document.diagnostics() {
    eprintln!("{:?}: {diagnostic}", diagnostic.range());
  }
  let output = if dump {
    dump_document(&document)
  } else {
    format_document(&document)
  };
  std::io::stdout().lock().write_all(output.as_bytes())?;
  Ok(())
}

fn dump_document(document: &Document<'_>) -> String {
  let mut output = String::new();
  dump_element(Element::Node(document.root()), 0, &mut output);
  output
}

fn dump_element(element: Element<'_, '_>, depth: usize, output: &mut String) {
  output.extend(std::iter::repeat_n(' ', depth * 2));
  match element {
    Element::Node(node) => {
      writeln!(output, "{:?}", node.kind())
        .expect("writing to a string cannot fail");
      for child in node.children() {
        dump_element(child, depth + 1, output);
      }
    },
    Element::Token(token) => {
      writeln!(output, "{:?}({:?})", token.kind(), token.text())
        .expect("writing to a string cannot fail");
    },
  }
}

fn format_document(document: &Document<'_>) -> String {
  let mut output = String::with_capacity(document.source().len());
  let mut depth = 0;
  let mut line_start = true;
  let mut pending_space = false;

  for index in 0..document.element_count() {
    let Some(Element::Token(token)) =
      document.element(ElementId::new(index as u32))
    else {
      continue;
    };
    let kind = token.kind();
    if matches!(kind, SyntaxKind::Whitespace | SyntaxKind::Eof) {
      pending_space |= kind == SyntaxKind::Whitespace;
      continue;
    }

    match kind {
      SyntaxKind::LeftBrace => {
        write_prefix(&mut output, depth, &mut line_start, pending_space);
        output.push_str(token.text());
        depth += 1;
        newline(&mut output, &mut line_start);
      },
      SyntaxKind::RightBrace => {
        depth = depth.saturating_sub(1);
        newline(&mut output, &mut line_start);
        write_prefix(&mut output, depth, &mut line_start, false);
        output.push_str(token.text());
      },
      SyntaxKind::Semicolon => {
        output.push_str(token.text());
        newline(&mut output, &mut line_start);
      },
      SyntaxKind::Assign
      | SyntaxKind::Equal
      | SyntaxKind::NotEqual
      | SyntaxKind::Less
      | SyntaxKind::Greater
      | SyntaxKind::LessEqual
      | SyntaxKind::GreaterEqual
      | SyntaxKind::Plus
      | SyntaxKind::Minus
      | SyntaxKind::Star
      | SyntaxKind::Slash
      | SyntaxKind::And
      | SyntaxKind::Or
      | SyntaxKind::Implication
      | SyntaxKind::Update
      | SyntaxKind::Concatenate
      | SyntaxKind::Question
      | SyntaxKind::PipeFrom
      | SyntaxKind::PipeInto => {
        write_prefix(&mut output, depth, &mut line_start, true);
        output.push_str(token.text());
        output.push(' ');
      },
      SyntaxKind::Colon | SyntaxKind::Comma => {
        output.push_str(token.text());
        output.push(' ');
      },
      SyntaxKind::LineComment => {
        write_prefix(&mut output, depth, &mut line_start, pending_space);
        output.push_str(token.text());
        newline(&mut output, &mut line_start);
      },
      _ => {
        write_prefix(&mut output, depth, &mut line_start, pending_space);
        output.push_str(token.text());
      },
    }
    pending_space = false;
  }

  while output.ends_with([' ', '\n']) {
    output.pop();
  }
  output.push('\n');
  output
}

fn write_prefix(
  output: &mut String,
  depth: usize,
  line_start: &mut bool,
  space: bool,
) {
  if *line_start {
    output.extend(std::iter::repeat_n(' ', depth * 2));
    *line_start = false;
  } else if space && !output.ends_with(' ') {
    output.push(' ');
  }
}

fn newline(output: &mut String, line_start: &mut bool) {
  while output.ends_with(' ') {
    output.pop();
  }
  if !*line_start {
    output.push('\n');
    *line_start = true;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn formats_nested_attribute_sets() {
    let document = parse_syntax(SAMPLE).expect("small source");
    assert_eq!(
      format_document(&document),
      "{\n  foo = 1;\n  # comments survive\n  nested = {\n    list = [1 2 \
       3];\n    enabled = true;\n  };\n}\n"
    );
  }

  #[test]
  fn dumps_nodes_and_lossless_trivia() {
    let document = parse_syntax("[ phlegm  ]").expect("small source");
    assert_eq!(
      dump_document(&document),
      "Root\n  List\n    LeftBracket(\"[\")\n    Literal\n      Whitespace(\" \
       \")\n      Identifier(\"phlegm\")\n    Whitespace(\"  \")\n    \
       RightBracket(\"]\")\n"
    );
  }
}
