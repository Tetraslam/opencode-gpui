use std::ops::Range;

use crate::markdown::{Block, Document, Inline, InlineStyle, Span, TaskState};

pub(crate) fn decorate(document: &mut Document) {
    for block in &mut document.blocks {
        match block {
            Block::Heading { content, .. } | Block::Paragraph { content, .. } => {
                decorate_inline(content);
            }
            Block::List { items, .. } => items.iter_mut().for_each(decorate_inline),
            Block::Table { header, rows } => {
                header.iter_mut().for_each(decorate_inline);
                rows.iter_mut().flatten().for_each(decorate_inline);
            }
            Block::Code { .. } | Block::Diagram { .. } | Block::Math { .. } | Block::Rule => {}
        }
    }
}

fn decorate_inline(inline: &mut Inline) {
    decorate_active_task(inline);
    let mut additions = Vec::new();
    for (range, raw) in tokens(&inline.text) {
        let token = if raw.starts_with("http://") || raw.starts_with("https://") {
            trim_url(raw)
        } else {
            trim_path(raw)
        };
        let range = range.start..range.start + token.len();
        if overlaps(&inline.spans, &range) {
            continue;
        }
        let mut style = InlineStyle::default();
        if is_url(token) {
            style.link = Some(token.to_owned());
        } else if is_path(token) {
            style.path = true;
        } else {
            continue;
        }
        additions.push(Span { range, style });
    }
    inline.spans.extend(additions);
    inline.spans.sort_unstable_by_key(|span| span.range.start);
}

fn decorate_active_task(inline: &mut Inline) {
    let state = if inline.text.starts_with("[.] ") {
        Some(TaskState::Active)
    } else if inline.text.starts_with("[-] ") {
        Some(TaskState::Cancelled)
    } else {
        None
    };
    if let Some(task) = state {
        inline.spans.push(Span {
            range: 0..4,
            style: InlineStyle {
                task: Some(task),
                ..InlineStyle::default()
            },
        });
    }
}

fn tokens(text: &str) -> impl Iterator<Item = (Range<usize>, &str)> {
    text.split_whitespace().filter_map(move |raw| {
        let raw_start = raw.as_ptr() as usize - text.as_ptr() as usize;
        let leading = raw.len() - raw.trim_start_matches(opening_punctuation).len();
        let candidate = raw.trim_start_matches(opening_punctuation);
        (!candidate.is_empty()).then(|| {
            let start = raw_start + leading;
            (start..start + candidate.len(), candidate)
        })
    })
}

fn opening_punctuation(character: char) -> bool {
    matches!(character, '(' | '[' | '{' | '<' | '"' | '\'')
}

fn trim_path(token: &str) -> &str {
    token.trim_end_matches(|character| {
        matches!(
            character,
            ')' | ']' | '}' | '>' | '"' | '\'' | ',' | ';' | ':' | '!' | '?'
        )
    })
}

fn trim_url(mut token: &str) -> &str {
    token = token.trim_end_matches(|character| {
        matches!(
            character,
            '.' | '>' | '"' | '\'' | ',' | ';' | ':' | '!' | '?'
        )
    });
    for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
        while token.ends_with(close) && token.matches(close).count() > token.matches(open).count() {
            token = token.trim_end_matches(close);
        }
    }
    token
}

fn overlaps(spans: &[Span], range: &Range<usize>) -> bool {
    spans
        .iter()
        .any(|span| span.range.start < range.end && range.start < span.range.end)
}

fn is_url(token: &str) -> bool {
    matches!(url::Url::parse(token), Ok(url) if matches!(url.scheme(), "http" | "https") && url.host().is_some())
}

fn is_path(token: &str) -> bool {
    if token.contains('@') || token.contains("://") {
        return false;
    }
    let valid = token.chars().all(|character| {
        character.is_alphanumeric() || matches!(character, '/' | '.' | '_' | '-' | '~')
    });
    if !valid {
        return false;
    }
    let rooted = token.starts_with('/')
        || token.starts_with('~')
        || token.starts_with("./")
        || token.starts_with("../");
    let segmented = token.contains('/') && !token.contains("//");
    let known_extension = token.rsplit_once('.').is_some_and(|(_, extension)| {
        matches!(
            extension.to_ascii_lowercase().as_str(),
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "py"
                | "md"
                | "toml"
                | "json"
                | "yaml"
                | "yml"
                | "sh"
                | "zsh"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cpp"
                | "h"
                | "hpp"
                | "css"
                | "html"
                | "sql"
                | "txt"
                | "lock"
        )
    });
    rooted || segmented || known_extension
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decorates_urls_and_conservative_paths_without_touching_emails() {
        let mut inline = Inline {
            text: "see https://example.com/a, src/main.rs and me@example.com".into(),
            spans: Vec::new(),
        };
        decorate_inline(&mut inline);
        assert!(inline.spans.iter().any(|span| span.style.link.is_some()));
        assert!(inline.spans.iter().any(|span| span.style.path));
        assert_eq!(inline.spans.len(), 2);
    }

    #[test]
    fn trims_sentence_punctuation_but_preserves_balanced_url_parentheses() {
        let mut inline = Inline {
            text: "see https://example.com/a_(b). next".into(),
            spans: Vec::new(),
        };
        decorate_inline(&mut inline);
        assert_eq!(
            inline.spans[0].style.link.as_deref(),
            Some("https://example.com/a_(b)")
        );
    }
}
