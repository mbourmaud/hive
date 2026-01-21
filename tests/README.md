# Hive Rust Tests

## TUI Testing

This project uses [ratatui](https://github.com/ratatui/ratatui) for TUI components and [insta](https://github.com/mitsuhiko/insta) for snapshot testing.

### Running TUI Tests

```bash
# Run all TUI tests
cargo test tui_

# Run TUI tests and review snapshots
cargo insta test --test tui_tests

# Accept all new snapshots
cargo insta test --accept --test tui_tests

# Review snapshots interactively
cargo insta review
```

### Test Structure

- `tui_tests.rs`: Contains all TUI snapshot tests
- `tests/snapshots/`: Contains snapshot files (.snap)

### What We Test

1. **Status Dashboard**: Single and multiple drone views
2. **Drone Detail View**: Progress, stories, and metadata
3. **Progress Bars**: Different percentage levels (0%, 25%, 50%, 75%, 100%)
4. **Blocked Drone View**: Error states and blocked reasons
5. **Session List**: Claude session viewer list
6. **Conversation View**: Message display with syntax highlighting
7. **Search Mode**: Search functionality in sessions

### How TUI Tests Work

TUI tests use `ratatui::backend::TestBackend` to render widgets to a memory buffer without requiring a real terminal. The buffer is then converted to a string and compared against saved snapshots.

```rust
fn render_to_string<F>(width: u16, height: u16, render_fn: F) -> String
where
    F: FnOnce(&mut ratatui::Frame),
{
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| {
        render_fn(f);
    }).unwrap();

    let buffer = terminal.backend().buffer().clone();
    buffer_to_string(&buffer, width, height)
}
```

### Updating Snapshots

When TUI components change, snapshots need to be updated:

```bash
# Run tests and see what changed
cargo insta test

# Review and accept changes interactively
cargo insta review

# Or accept all changes automatically
cargo insta test --accept
```

### CI Integration

TUI tests run in CI without requiring a real terminal. The `ci-rust.yml` workflow runs all tests including TUI tests on every pull request.
