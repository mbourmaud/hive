// Parse log line and return a summary for display
pub(crate) fn parse_log_summary(line: &str, max_width: usize) -> String {
    // Try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // Extract useful info from stream-json format
        let msg_type = json.get("type").and_then(|v| v.as_str()).unwrap_or("?");

        let summary = match msg_type {
            "assistant" => {
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    if let Some(first) = content.first() {
                        if let Some(text) = first.get("text").and_then(|t| t.as_str()) {
                            let short = text.chars().take(80).collect::<String>();
                            format!("\u{1f4ac} {}", short.replace('\n', " "))
                        } else if let Some(name) = first.get("name").and_then(|n| n.as_str()) {
                            // Get tool input for more context
                            let context = if let Some(input) = first.get("input") {
                                if let Some(file) = input.get("file_path").and_then(|f| f.as_str())
                                {
                                    // Extract just filename
                                    file.rsplit('/').next().unwrap_or(file).to_string()
                                } else if let Some(cmd) =
                                    input.get("command").and_then(|c| c.as_str())
                                {
                                    cmd.chars().take(40).collect::<String>()
                                } else if let Some(pattern) =
                                    input.get("pattern").and_then(|p| p.as_str())
                                {
                                    format!("/{}/", pattern.chars().take(30).collect::<String>())
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            if context.is_empty() {
                                format!("\u{1f527} {}", name)
                            } else {
                                format!("\u{1f527} {} \u{2192} {}", name, context)
                            }
                        } else {
                            "\u{1f4ac} assistant".to_string()
                        }
                    } else {
                        "\u{1f4ac} assistant".to_string()
                    }
                } else {
                    "\u{1f4ac} assistant".to_string()
                }
            }
            "user" => {
                // User messages are typically tool results
                if let Some(content) = json
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_array())
                {
                    if let Some(first) = content.first() {
                        let tool_type = first.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if tool_type == "tool_result" {
                            // Check tool_use_result for details
                            if let Some(result) = json.get("tool_use_result") {
                                // Edit result
                                if let Some(file) = result.get("filePath").and_then(|f| f.as_str())
                                {
                                    let filename = file.rsplit('/').next().unwrap_or(file);
                                    return truncate_summary(
                                        &format!("\u{2713} Edit \u{2192} {}", filename),
                                        max_width,
                                    );
                                }
                                // Bash result
                                if let Some(stdout) = result.get("stdout").and_then(|s| s.as_str())
                                {
                                    let short = stdout
                                        .lines()
                                        .next()
                                        .unwrap_or("")
                                        .chars()
                                        .take(50)
                                        .collect::<String>();
                                    return truncate_summary(
                                        &format!("\u{2713} Bash \u{2192} {}", short),
                                        max_width,
                                    );
                                }
                                // Read result
                                if result.get("content").is_some() {
                                    if let Some(file) =
                                        result.get("filePath").and_then(|f| f.as_str())
                                    {
                                        let filename = file.rsplit('/').next().unwrap_or(file);
                                        return truncate_summary(
                                            &format!("\u{2713} Read \u{2192} {}", filename),
                                            max_width,
                                        );
                                    }
                                }
                                // Glob/Grep result
                                if let Some(files) = result.get("files").and_then(|f| f.as_array())
                                {
                                    return truncate_summary(
                                        &format!("\u{2713} Found {} files", files.len()),
                                        max_width,
                                    );
                                }
                            }
                            // Fallback: get content text
                            if let Some(content_text) =
                                first.get("content").and_then(|c| c.as_str())
                            {
                                let short = content_text.chars().take(50).collect::<String>();
                                return truncate_summary(
                                    &format!("\u{2713} {}", short.replace('\n', " ")),
                                    max_width,
                                );
                            }
                        }
                    }
                }
                "\u{1f464} user".to_string()
            }
            "result" => {
                if let Some(result) = json.get("result").and_then(|r| r.as_str()) {
                    let short = result.chars().take(60).collect::<String>();
                    format!("\u{2713} {}", short.replace('\n', " "))
                } else {
                    "\u{2713} result".to_string()
                }
            }
            "system" => {
                let subtype = json.get("subtype").and_then(|s| s.as_str()).unwrap_or("");
                match subtype {
                    "init" => "\u{2699} Session started".to_string(),
                    _ => format!("\u{2699} {}", subtype),
                }
            }
            "error" => {
                if let Some(err) = json.get("error").and_then(|e| e.as_str()) {
                    format!("\u{274c} {}", err)
                } else {
                    "\u{274c} error".to_string()
                }
            }
            _ => format!("[{}]", msg_type),
        };

        truncate_summary(&summary, max_width)
    } else {
        // Not JSON, show raw line truncated
        truncate_summary(line, max_width)
    }
}

pub(crate) fn truncate_summary(s: &str, max_width: usize) -> String {
    if s.len() > max_width {
        format!("{}...", &s[..max_width.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

// Pretty-print JSON with indentation and word wrap
pub(crate) fn pretty_print_json(line: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // Use custom formatting for better readability
        format_json_value(&json, 0)
    } else {
        line.to_string()
    }
}

// Recursively format JSON with proper indentation and no truncation
pub(crate) fn format_json_value(value: &serde_json::Value, indent: usize) -> String {
    let indent_str = "  ".repeat(indent);
    let next_indent = "  ".repeat(indent + 1);

    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // For long strings, wrap them
            if s.len() > 80 {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n");
                format!("\"{}\"", escaped)
            } else {
                format!("{:?}", s)
            }
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr
                    .iter()
                    .map(|v| format!("{}{}", next_indent, format_json_value(v, indent + 1)))
                    .collect();
                format!("[\n{}\n{}]", items.join(",\n"), indent_str)
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| {
                        let formatted_value = format_json_value(v, indent + 1);
                        format!("{}\"{}\": {}", next_indent, k, formatted_value)
                    })
                    .collect();
                format!("{{\n{}\n{}}}", items.join(",\n"), indent_str)
            }
        }
    }
}
