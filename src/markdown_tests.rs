use super::*;

#[test]
fn parses_gfm_blocks_and_inline_styles() {
    let document =
        parse("# title\n\n**bold** and `code`\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done");

    assert!(matches!(
        document.blocks[0],
        Block::Heading { level: 1, .. }
    ));
    let Block::Paragraph { content, .. } = &document.blocks[1] else {
        panic!("expected paragraph");
    };
    assert_eq!(content.text, "bold and code");
    assert!(content.spans.iter().any(|span| span.style.bold));
    assert!(content.spans.iter().any(|span| span.style.code));
    assert!(matches!(document.blocks[2], Block::Table { .. }));
    assert!(matches!(document.blocks[3], Block::List { .. }));
}

#[test]
fn keeps_partial_streaming_fences_visible() {
    let document = parse("```rust\nfn main() {");

    assert_eq!(
        document.blocks,
        [Block::Code {
            language: "rust".into(),
            content: "fn main() {".into(),
        }]
    );
}

#[test]
fn identifies_diagram_fences() {
    let document = parse("```mermaid\ngraph TD; A-->B\n```");
    assert!(matches!(document.blocks[0], Block::Diagram { .. }));
}

#[test]
fn parses_dollar_and_tex_math_delimiters() {
    let document = parse("before $x^2$ and \\(y+1\\)\n\n$$z=3$$\n\n\\[w=4\\]");

    let Block::Paragraph { content, .. } = &document.blocks[0] else {
        panic!("expected paragraph");
    };
    let formulas = content
        .spans
        .iter()
        .filter_map(|span| span.style.math.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(formulas, ["x^2", "y+1"]);
    assert_eq!(content.text, "before $x^2$ and $y+1$");
    assert!(matches!(
        &document.blocks[1],
        Block::Math { content } if content == "z=3"
    ));
    assert!(matches!(
        &document.blocks[2],
        Block::Math { content } if content == "w=4"
    ));
}

#[test]
fn exposes_deduplicated_render_requests() {
    let document = parse("$x$ and $x$\n\n```mermaid\nflowchart LR\nA-->B\n```");
    let requests = document.render_requests();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .any(|request| { request.kind == RenderKind::MathInline && request.source == "x" })
    );
    assert!(requests.iter().any(|request| {
        request.kind == RenderKind::Mermaid && request.source.contains("flowchart")
    }));
}
