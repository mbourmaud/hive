use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

pub fn render_markdown(input: &str) -> Text<'static> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(input, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_spans: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut list_depth: usize = 0;
    let mut ordered_index: Option<u64> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    style_stack.push(style);
                    let prefix = "#".repeat(level as usize);
                    current_spans.push(Span::styled(format!("{} ", prefix), style));
                }
                Tag::Paragraph => {}
                Tag::Strong => {
                    let style = current_style(&style_stack).add_modifier(Modifier::BOLD);
                    style_stack.push(style);
                }
                Tag::Emphasis => {
                    let style = current_style(&style_stack).add_modifier(Modifier::ITALIC);
                    style_stack.push(style);
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    // Flush current line
                    if !current_spans.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_spans)));
                    }
                    // Code block header
                    let label = if code_lang.is_empty() {
                        " code ".to_string()
                    } else {
                        format!(" {} ", code_lang)
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!("---[{}]---", label),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
                Tag::List(start) => {
                    list_depth += 1;
                    ordered_index = start;
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    let bullet = if let Some(idx) = ordered_index {
                        let s = format!("{}{}. ", indent, idx);
                        ordered_index = Some(idx + 1);
                        s
                    } else {
                        format!("{}- ", indent)
                    };
                    current_spans.push(Span::styled(bullet, Style::default().fg(Color::Yellow)));
                }
                Tag::BlockQuote(_) => {
                    current_spans.push(Span::styled(
                        "| ",
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ));
                    let style = Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC);
                    style_stack.push(style);
                }
                Tag::Link { dest_url, .. } => {
                    let style = Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::UNDERLINED);
                    style_stack.push(style);
                    // Store the URL for the end event
                    current_spans.push(Span::raw(format!("[link:{}]", dest_url)));
                }
                _ => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                TagEnd::Paragraph => {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                    lines.push(Line::from(""));
                }
                TagEnd::Strong | TagEnd::Emphasis => {
                    style_stack.pop();
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    code_lang.clear();
                    lines.push(Line::from(vec![Span::styled(
                        "----------",
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    if list_depth == 0 {
                        ordered_index = None;
                    }
                }
                TagEnd::Item => {
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                TagEnd::BlockQuote(_) => {
                    style_stack.pop();
                    lines.push(Line::from(std::mem::take(&mut current_spans)));
                }
                TagEnd::Link => {
                    style_stack.pop();
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    let code_style = Style::default().fg(Color::Green).bg(Color::Rgb(40, 42, 54));
                    for line_str in text.split('\n') {
                        if !line_str.is_empty() {
                            lines.push(Line::from(vec![
                                Span::styled("  ", code_style),
                                Span::styled(line_str.to_string(), code_style),
                            ]));
                        }
                    }
                } else {
                    let style = current_style(&style_stack);
                    current_spans.push(Span::styled(text.to_string(), style));
                }
            }
            Event::Code(code) => {
                current_spans.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Green).bg(Color::Rgb(40, 42, 54)),
                ));
            }
            Event::SoftBreak | Event::HardBreak => {
                lines.push(Line::from(std::mem::take(&mut current_spans)));
            }
            Event::Rule => {
                lines.push(Line::from(vec![Span::styled(
                    "────────────────────",
                    Style::default().fg(Color::DarkGray),
                )]));
            }
            _ => {}
        }
    }

    // Flush remaining spans
    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }

    Text::from(lines)
}

fn current_style(stack: &[Style]) -> Style {
    stack.last().copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let text = render_markdown("Hello, world!");
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_bold_text() {
        let text = render_markdown("**bold text**");
        assert!(!text.lines.is_empty());
        let line = &text.lines[0];
        let has_bold = line
            .spans
            .iter()
            .any(|s| s.style.add_modifier.contains(Modifier::BOLD));
        assert!(has_bold);
    }

    #[test]
    fn test_code_block() {
        let text = render_markdown("```rust\nfn main() {}\n```");
        let text_str: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(text_str.contains("rust"));
        assert!(text_str.contains("fn main()"));
    }

    #[test]
    fn test_inline_code() {
        let text = render_markdown("use `println!` macro");
        let text_str: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(text_str.contains("`println!`"));
    }

    #[test]
    fn test_heading() {
        let text = render_markdown("# Title\n\nContent");
        let text_str: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(text_str.contains("#"));
        assert!(text_str.contains("Title"));
    }

    #[test]
    fn test_list() {
        let text = render_markdown("- item 1\n- item 2\n- item 3");
        let text_str: String = text
            .lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(text_str.contains("item 1"));
        assert!(text_str.contains("item 2"));
    }
}
