use std::panic::{AssertUnwindSafe, catch_unwind};

use ratex_layout::{LayoutOptions, layout, to_display_list};
use ratex_parser::parse;
use ratex_svg::{SvgOptions, render_to_svg};
use ratex_types::{color::Color, math_style::MathStyle};

use crate::{
    markdown::{RenderKind, RenderRequest},
    theme::color,
};

pub const MAX_INPUT_BYTES: usize = 16 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct RenderedSvg {
    pub bytes: Vec<u8>,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("source exceeds 16 KiB")]
    InputTooLarge,
    #[error("SVG exceeds 8 MiB")]
    OutputTooLarge,
    #[error("render failed: {0}")]
    Failed(String),
    #[error("renderer panicked")]
    Panicked,
    #[error("renderer returned an invalid SVG size")]
    InvalidSize,
}

pub fn render(request: &RenderRequest) -> Result<RenderedSvg, RenderError> {
    if request.source.len() > MAX_INPUT_BYTES {
        return Err(RenderError::InputTooLarge);
    }
    let rendered = catch_unwind(AssertUnwindSafe(|| match request.kind {
        RenderKind::Mermaid => render_mermaid(&request.source),
        RenderKind::MathInline => render_math(&request.source, false),
        RenderKind::MathDisplay => render_math(&request.source, true),
    }))
    .map_err(|_| RenderError::Panicked)??;
    if rendered.len() > MAX_OUTPUT_BYTES {
        return Err(RenderError::OutputTooLarge);
    }
    let (width, height) = svg_size(&rendered).ok_or(RenderError::InvalidSize)?;
    Ok(RenderedSvg {
        bytes: rendered.into_bytes(),
        width,
        height,
    })
}

fn render_mermaid(source: &str) -> Result<String, RenderError> {
    let themed = format!("{}\n{source}", mermaid_init());
    sebastian::render_diagram(&themed, "markdown-mermaid")
        .map_err(|error| RenderError::Failed(error.to_string()))
}

fn mermaid_init() -> String {
    format!(
        "%%{{init: {{\"theme\":\"dark\",\"htmlLabels\":false,\"flowchart\":{{\"htmlLabels\":false}},\"themeVariables\":{{\"background\":\"{}\",\"primaryColor\":\"{}\",\"primaryTextColor\":\"{}\",\"primaryBorderColor\":\"{}\",\"lineColor\":\"{}\",\"secondaryColor\":\"{}\",\"tertiaryColor\":\"{}\"}}}}}}%%",
        hex(color::BASE),
        hex(color::SURFACE),
        hex(color::TEXT_BRIGHT),
        hex(color::ACCENT),
        hex(color::TEXT_DIM),
        hex(color::ELEVATED),
        hex(color::SELECTED),
    )
}

fn render_math(source: &str, display: bool) -> Result<String, RenderError> {
    let ast = parse(source).map_err(|error| RenderError::Failed(error.to_string()))?;
    let style = if display {
        MathStyle::Display
    } else {
        MathStyle::Text
    };
    let text = color::TEXT;
    let foreground = Color::new(
        ((text >> 16) & 0xff) as f32 / 255.0,
        ((text >> 8) & 0xff) as f32 / 255.0,
        (text & 0xff) as f32 / 255.0,
        1.0,
    );
    let options = LayoutOptions::default()
        .with_style(style)
        .with_color(foreground);
    let list = to_display_list(&layout(&ast, &options));
    Ok(render_to_svg(
        &list,
        &SvgOptions {
            font_size: if display { 21.0 } else { 17.0 },
            padding: if display { 4.0 } else { 1.0 },
            stroke_width: 1.0,
            embed_glyphs: true,
            font_dir: String::new(),
        },
    ))
}

fn hex(value: u32) -> String {
    format!("#{:06x}", value & 0x00ff_ffff)
}

fn svg_size(svg: &str) -> Option<(f32, f32)> {
    let tag_end = svg.find('>')?;
    let tag = &svg[..tag_end];
    if let Some(view_box) = attribute(tag, "viewBox") {
        let values = view_box
            .split(|character: char| character.is_ascii_whitespace() || character == ',')
            .filter(|value| !value.is_empty())
            .map(str::parse::<f32>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if values.len() == 4 && values[2].is_finite() && values[3].is_finite() {
            return Some((values[2].max(1.0), values[3].max(1.0)));
        }
    }
    Some((dimension(tag, "width")?, dimension(tag, "height")?))
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let start = tag.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = &tag[start..];
    Some(&rest[..rest.find('"')?])
}

fn dimension(tag: &str, name: &str) -> Option<f32> {
    let value = attribute(tag, name)?;
    value
        .trim_end_matches(|character: char| character.is_ascii_alphabetic())
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_embedded_math_svg() {
        let rendered = render(&RenderRequest {
            kind: RenderKind::MathInline,
            source: r"x^2 + \sqrt{y}".into(),
        })
        .expect("valid math");
        let svg = String::from_utf8(rendered.bytes).expect("UTF-8 SVG");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<path") || svg.contains("<image"));
        assert!(rendered.width > rendered.height);
    }

    #[test]
    fn renders_dark_mermaid_svg() {
        let rendered = render(&RenderRequest {
            kind: RenderKind::Mermaid,
            source: "flowchart LR\nA --> B".into(),
        })
        .expect("valid Mermaid");
        let svg = String::from_utf8(rendered.bytes).expect("UTF-8 SVG");
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("#3d405a") && svg.contains("#9299bd"));
        assert!(!svg.contains("<foreignObject"));
    }

    #[test]
    fn rejects_oversized_input_before_rendering() {
        let error = render(&RenderRequest {
            kind: RenderKind::MathDisplay,
            source: "x".repeat(MAX_INPUT_BYTES + 1),
        })
        .expect_err("oversized input must fail");
        assert!(matches!(error, RenderError::InputTooLarge));
    }
}
