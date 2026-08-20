pub(crate) fn normalize_delimiters(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut fenced: Option<(char, usize)> = None;
    for segment in source.split_inclusive('\n') {
        let line = segment.strip_suffix('\n').unwrap_or(segment);
        let trimmed = line.trim_start();
        let fence = fence_marker(trimmed);
        if let Some((character, length)) = fenced {
            output.push_str(segment);
            if fence.is_some_and(|candidate| candidate.0 == character && candidate.1 >= length) {
                fenced = None;
            }
            continue;
        }
        if let Some(marker) = fence {
            fenced = Some(marker);
            output.push_str(segment);
            continue;
        }
        output.push_str(&normalize_line(line));
        if segment.ends_with('\n') {
            output.push('\n');
        }
    }
    output
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let character = line.chars().next()?;
    if !matches!(character, '`' | '~') {
        return None;
    }
    let length = line.chars().take_while(|value| *value == character).count();
    (length >= 3).then_some((character, length))
}

fn normalize_line(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut index = 0;
    let mut code_ticks = None;
    while index < line.len() {
        let rest = &line[index..];
        if rest.starts_with('`') {
            let count = rest.bytes().take_while(|byte| *byte == b'`').count();
            code_ticks = match code_ticks {
                Some(open) if open == count => None,
                None => Some(count),
                current => current,
            };
            output.push_str(&rest[..count]);
            index += count;
            continue;
        }
        if code_ticks.is_none() {
            let replacement = if rest.starts_with("\\(") || rest.starts_with("\\)") {
                Some("$")
            } else if rest.starts_with("\\[") || rest.starts_with("\\]") {
                Some("$$")
            } else {
                None
            };
            if let Some(replacement) = replacement {
                output.push_str(replacement);
                index += 2;
                continue;
            }
        }
        let character = rest.chars().next().expect("nonempty remainder");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

#[cfg(test)]
mod tests {
    use super::normalize_delimiters;

    #[test]
    fn normalizes_tex_delimiters_but_not_code() {
        let source = "\\(x\\) `\\(code\\)`\n```tex\n\\[raw\\]\n```\n\\[y\\]";
        assert_eq!(
            normalize_delimiters(source),
            "$x$ `\\(code\\)`\n```tex\n\\[raw\\]\n```\n$$y$$"
        );
    }
}
