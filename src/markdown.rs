use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub use crate::markdown_types::{
    Block, Document, Inline, InlineStyle, RenderKind, RenderRequest, Span, TaskState,
};

#[cfg(test)]
#[path = "markdown_tests.rs"]
mod tests;

#[derive(Default)]
struct Builder {
    document: Document,
    inline: Inline,
    style: InlineStyle,
    heading: Option<u8>,
    quoted: usize,
    code: Option<(String, String)>,
    list: Option<(Option<u64>, Vec<Inline>)>,
    table: Option<TableBuilder>,
}

#[derive(Default)]
struct TableBuilder {
    rows: Vec<Vec<Inline>>,
    row: Vec<Inline>,
}

#[must_use]
pub fn parse(source: &str) -> Document {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_MATH;
    let normalized = crate::markdown_math::normalize_delimiters(source);
    let mut builder = Builder::default();
    for event in Parser::new_ext(&normalized, options) {
        builder.event(event);
    }
    builder.finish();
    crate::markdown_inline::decorate(&mut builder.document);
    builder.document
}

impl Builder {
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::InlineHtml(html) if html.trim().to_ascii_lowercase().starts_with("<kbd") => {
                self.style.kbd = true;
            }
            Event::InlineHtml(html) if html.eq_ignore_ascii_case("</kbd>") => {
                self.style.kbd = false;
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.push_text(&text);
            }
            Event::Code(text) => {
                let previous = self.style.code;
                self.style.code = true;
                self.push_text(&text);
                self.style.code = previous;
            }
            Event::InlineMath(math) => self.push_math(&math),
            Event::DisplayMath(math) => self.push_display_math(&math),
            Event::SoftBreak => self.push_text(" "),
            Event::HardBreak => self.push_text("\n"),
            Event::Rule => self.document.blocks.push(Block::Rule),
            Event::TaskListMarker(checked) => {
                self.style.task = Some(if checked {
                    TaskState::Checked
                } else {
                    TaskState::Pending
                });
                self.push_text(if checked { "[x] " } else { "[ ] " });
                self.style.task = None;
            }
            Event::FootnoteReference(reference) => self.push_text(&format!("[{reference}]")),
        }
    }

    fn start(&mut self, tag: Tag<'_>) {
        match tag {
            Tag::Heading { level, .. } => self.heading = Some(heading_level(level)),
            Tag::BlockQuote(_) => self.quoted += 1,
            Tag::CodeBlock(kind) => {
                let language = match kind {
                    CodeBlockKind::Indented => String::new(),
                    CodeBlockKind::Fenced(language) => language.into_string(),
                };
                self.code = Some((language, String::new()));
            }
            Tag::List(start) => self.list = Some((start, Vec::new())),
            Tag::Table(_) => self.table = Some(TableBuilder::default()),
            Tag::Emphasis => self.style.italic = true,
            Tag::Strong => self.style.bold = true,
            Tag::Strikethrough => self.style.strike = true,
            Tag::Link { dest_url, .. } => self.style.link = Some(dest_url.into_string()),
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph => self.flush_paragraph(),
            TagEnd::Heading(_) => {
                let content = std::mem::take(&mut self.inline);
                self.document.blocks.push(Block::Heading {
                    level: self.heading.take().unwrap_or(1),
                    content,
                });
            }
            TagEnd::BlockQuote(_) => self.quoted = self.quoted.saturating_sub(1),
            TagEnd::CodeBlock => self.flush_code(),
            TagEnd::Item => {
                if let Some((_, items)) = &mut self.list
                    && !self.inline.text.is_empty()
                {
                    items.push(std::mem::take(&mut self.inline));
                }
            }
            TagEnd::List(_) => {
                if let Some((start, items)) = self.list.take() {
                    self.document.blocks.push(Block::List { start, items });
                }
            }
            TagEnd::TableCell => {
                if let Some(table) = &mut self.table {
                    table.row.push(std::mem::take(&mut self.inline));
                }
            }
            TagEnd::TableRow => {
                if let Some(table) = &mut self.table {
                    table.rows.push(std::mem::take(&mut table.row));
                }
            }
            TagEnd::Table => self.flush_table(),
            TagEnd::Emphasis => self.style.italic = false,
            TagEnd::Strong => self.style.bold = false,
            TagEnd::Strikethrough => self.style.strike = false,
            TagEnd::Link => self.style.link = None,
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        if let Some((_, content)) = &mut self.code {
            content.push_str(text);
            return;
        }
        let start = self.inline.text.len();
        self.inline.text.push_str(text);
        let end = self.inline.text.len();
        if start != end && self.style != InlineStyle::default() {
            self.inline.spans.push(Span {
                range: start..end,
                style: self.style.clone(),
            });
        }
    }

    fn push_math(&mut self, math: &str) {
        let previous = self.style.math.replace(math.to_owned());
        self.push_text(&format!("${math}$"));
        self.style.math = previous;
    }

    fn push_display_math(&mut self, math: &str) {
        if self.list.is_some() || self.table.is_some() || self.heading.is_some() {
            self.push_math(math);
            return;
        }
        self.flush_paragraph();
        self.document.blocks.push(Block::Math {
            content: math.to_owned(),
        });
    }

    fn flush_paragraph(&mut self) {
        if self.inline.text.is_empty() {
            return;
        }
        if self.list.is_some() {
            return;
        }
        self.document.blocks.push(Block::Paragraph {
            content: std::mem::take(&mut self.inline),
            quoted: self.quoted > 0,
        });
    }

    fn flush_code(&mut self) {
        let Some((language, content)) = self.code.take() else {
            return;
        };
        let diagram = matches!(
            language.as_str(),
            "mermaid" | "dot" | "graphviz" | "plantuml"
        );
        self.document.blocks.push(if diagram {
            Block::Diagram { language, content }
        } else {
            Block::Code { language, content }
        });
    }

    fn flush_table(&mut self) {
        let Some(mut table) = self.table.take() else {
            return;
        };
        let header = if table.rows.is_empty() {
            Vec::new()
        } else {
            table.rows.remove(0)
        };
        self.document.blocks.push(Block::Table {
            header,
            rows: table.rows,
        });
    }

    fn finish(&mut self) {
        if self.code.is_some() {
            self.flush_code();
        }
        if !self.inline.text.is_empty() {
            self.flush_paragraph();
        }
    }
}

const fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
