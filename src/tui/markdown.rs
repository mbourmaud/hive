use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Convert markdown text into styled ratatui Lines
pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(text, options);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();

    let mut bold = false;
    let mut italic = false;
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut in_heading = false;
    let mut list_depth: usize = 0;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong => bold = true,
                Tag::Emphasis => italic = true,
                Tag::CodeBlock(kind) => {
                    // Flush current line
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    in_code_block = true;
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    // Add code block header
                    let label = if code_lang.is_empty() {
                        " code ".to_string()
                    } else {
                        format!(" {} ", code_lang)
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!("  \u{250c}\u{2500}{}\u{2500}", label),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
                Tag::Heading { level, .. } => {
                    in_heading = true;
                    let prefix = match level {
                        pulldown_cmark::HeadingLevel::H1 => "# ",
                        pulldown_cmark::HeadingLevel::H2 => "## ",
                        pulldown_cmark::HeadingLevel::H3 => "### ",
                        _ => "#### ",
                    };
                    current_spans.push(Span::styled(
                        prefix.to_string(),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ));
                }
                Tag::List(_) => {
                    list_depth += 1;
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    current_spans.push(Span::styled(
                        format!("{}\u{2022} ", indent),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                Tag::Paragraph => {}
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Strong => bold = false,
                TagEnd::Emphasis => italic = false,
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    code_lang.clear();
                    lines.push(Line::from(vec![Span::styled(
                        "  \u{2514}\u{2500}\u{2500}\u{2500}".to_string(),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
                TagEnd::Heading(_) => {
                    in_heading = false;
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                TagEnd::Paragraph => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Item => {
                    if !current_spans.is_empty() {
                        lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                    }
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    // Render code block lines with background-like indent
                    for code_line in text.lines() {
                        lines.push(Line::from(vec![
                            Span::styled(
                                "  \u{2502} ".to_string(),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                code_line.to_string(),
                                Style::default().fg(Color::Yellow),
                            ),
                        ]));
                    }
                } else {
                    let mut style = Style::default();
                    if bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if italic {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if in_heading {
                        style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                    }
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_spans.is_empty() {
                    lines.push(Line::from(current_spans.drain(..).collect::<Vec<_>>()));
                }
            }
            _ => {}
        }
    }

    // Flush remaining spans
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    // Remove trailing empty lines
    while lines.last().map_or(false, |l| l.spans.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        lines.push(Line::from(text.to_string()));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let lines = render_markdown("Hello world");
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_bold() {
        let lines = render_markdown("**bold text**");
        assert!(!lines.is_empty());
        // Check that some span has BOLD modifier
        let has_bold = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::BOLD))
        });
        assert!(has_bold);
    }

    #[test]
    fn test_italic() {
        let lines = render_markdown("*italic text*");
        assert!(!lines.is_empty());
        let has_italic = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.style.add_modifier.contains(Modifier::ITALIC))
        });
        assert!(has_italic);
    }

    #[test]
    fn test_inline_code() {
        let lines = render_markdown("Use `foo()` here");
        assert!(!lines.is_empty());
        let has_code = lines.iter().any(|l| {
            l.spans
                .iter()
                .any(|s| s.content.contains("`foo()`"))
        });
        assert!(has_code);
    }

    #[test]
    fn test_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let lines = render_markdown(md);
        // Should have header, code line, footer
        assert!(lines.len() >= 3);
        // Check code line has yellow color
        let has_code_content = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("fn main()")));
        assert!(has_code_content);
    }

    #[test]
    fn test_heading() {
        let lines = render_markdown("# Title");
        assert!(!lines.is_empty());
        let has_prefix = lines
            .iter()
            .any(|l| l.spans.iter().any(|s| s.content.contains("# ")));
        assert!(has_prefix);
    }

    #[test]
    fn test_list() {
        let md = "- item 1\n- item 2";
        let lines = render_markdown(md);
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_empty_input() {
        let lines = render_markdown("");
        assert!(!lines.is_empty()); // Should at least have one line
    }
}
