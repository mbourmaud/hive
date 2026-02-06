use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use super::theme::Theme;

/// Convert markdown text to ratatui Lines with appropriate styling
pub fn render_markdown(text: &str, theme: &Theme) -> Vec<Line<'static>> {
    let parser = Parser::new(text);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut bold_active = false;
    let mut italic_active = false;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong => bold_active = true,
                Tag::Emphasis => italic_active = true,
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(lang) = kind {
                        code_block_lang = lang.to_string();
                    }
                }
                Tag::Heading(_, _, _) => {
                    // Headings are bold and on new line
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    bold_active = true;
                }
                Tag::List(_) | Tag::Item => {
                    // Start new line for list items
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Tag::Paragraph => {
                    // Ensure previous paragraph is closed
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                Tag::Strong => bold_active = false,
                Tag::Emphasis => italic_active = false,
                Tag::CodeBlock(_) => {
                    // Finish code block
                    if !code_block_lang.is_empty() {
                        lines.push(Line::from(vec![Span::styled(
                            format!("\u{250c}\u{2500} {} ", code_block_lang),
                            Style::default().fg(theme.code_border),
                        )]));
                    } else {
                        lines.push(Line::from(vec![Span::styled(
                            "\u{250c}\u{2500}",
                            Style::default().fg(theme.code_border),
                        )]));
                    }
                    for code_line in &code_block_lines {
                        lines.push(Line::from(vec![
                            Span::styled(
                                "\u{2502} ",
                                Style::default().fg(theme.code_border),
                            ),
                            Span::styled(
                                code_line.clone(),
                                Style::default()
                                    .fg(theme.code_fg)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]));
                    }
                    lines.push(Line::from(vec![Span::styled(
                        "\u{2514}\u{2500}",
                        Style::default().fg(theme.code_border),
                    )]));
                    in_code_block = false;
                    code_block_lang.clear();
                    code_block_lines.clear();
                }
                Tag::Heading(_, _, _) => {
                    bold_active = false;
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Tag::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    // Add spacing after paragraph
                    lines.push(Line::from(vec![]));
                }
                Tag::List(_) | Tag::Item => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    // Add text to code block buffer
                    code_block_lines.extend(text.lines().map(|s| s.to_string()));
                } else {
                    // Apply inline styling
                    let mut style = Style::default();
                    if bold_active {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if italic_active {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    current_line.push(Span::styled(text.to_string(), style));
                }
            }
            Event::Code(code) => {
                // Inline code
                current_line.push(Span::styled(
                    format!("`{}`", code),
                    Style::default()
                        .fg(theme.inline_code_fg)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
            }
            _ => {}
        }
    }

    // Flush any remaining text
    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_theme() -> Theme {
        Theme::dark()
    }

    #[test]
    fn test_bold_rendering() {
        let text = "This is **bold** text";
        let lines = render_markdown(text, &test_theme());
        assert!(!lines.is_empty());
        // Verify the line contains styled spans
        assert!(lines[0].spans.len() >= 2);
    }

    #[test]
    fn test_italic_rendering() {
        let text = "This is *italic* text";
        let lines = render_markdown(text, &test_theme());
        assert!(!lines.is_empty());
        assert!(lines[0].spans.len() >= 2);
    }

    #[test]
    fn test_inline_code() {
        let text = "Use `cargo build` to compile";
        let lines = render_markdown(text, &test_theme());
        assert!(!lines.is_empty());
        // Should have at least one span with code
        assert!(lines[0].spans.iter().any(|s| s.content.contains('`')));
    }

    #[test]
    fn test_code_block() {
        let text = "```rust\nfn main() {}\n```";
        let lines = render_markdown(text, &test_theme());
        // Code blocks should produce multiple lines (header, content, footer)
        assert!(lines.len() >= 3);
    }
}
