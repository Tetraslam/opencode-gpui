use std::ops::Range;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        content: Inline,
    },
    Paragraph {
        content: Inline,
        quoted: bool,
    },
    Code {
        language: String,
        content: String,
    },
    Diagram {
        language: String,
        content: String,
    },
    Math {
        content: String,
    },
    List {
        start: Option<u64>,
        items: Vec<Inline>,
    },
    Table {
        header: Vec<Inline>,
        rows: Vec<Vec<Inline>>,
    },
    Rule,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inline {
    pub text: String,
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub range: Range<usize>,
    pub style: InlineStyle,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
    pub path: bool,
    pub kbd: bool,
    pub task: Option<TaskState>,
    pub link: Option<String>,
    pub math: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Pending,
    Active,
    Checked,
    Cancelled,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RenderRequest {
    pub kind: RenderKind,
    pub source: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RenderKind {
    Mermaid,
    MathInline,
    MathDisplay,
}

impl Document {
    #[must_use]
    pub fn render_requests(&self) -> Vec<RenderRequest> {
        let mut requests = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Diagram { language, content }
                    if language.trim().eq_ignore_ascii_case("mermaid") =>
                {
                    requests.push(RenderRequest {
                        kind: RenderKind::Mermaid,
                        source: content.clone(),
                    });
                }
                Block::Math { content } => requests.push(RenderRequest {
                    kind: RenderKind::MathDisplay,
                    source: content.clone(),
                }),
                Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                    inline_requests(content, &mut requests);
                }
                Block::List { items, .. } => {
                    for item in items {
                        inline_requests(item, &mut requests);
                    }
                }
                Block::Table { header, rows } => {
                    header
                        .iter()
                        .chain(rows.iter().flatten())
                        .for_each(|cell| inline_requests(cell, &mut requests));
                }
                _ => {}
            }
        }
        requests.sort_unstable_by(|left, right| {
            left.kind
                .cmp(&right.kind)
                .then_with(|| left.source.cmp(&right.source))
        });
        requests.dedup();
        requests
    }
}

fn inline_requests(inline: &Inline, requests: &mut Vec<RenderRequest>) {
    requests.extend(inline.spans.iter().filter_map(|span| {
        span.style.math.as_ref().map(|source| RenderRequest {
            kind: RenderKind::MathInline,
            source: source.clone(),
        })
    }));
}
