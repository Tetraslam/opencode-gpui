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
