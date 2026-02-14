use super::*;

#[test]
fn test_show_team_conversation_no_panic() {
    // Basic smoke test - just ensure it doesn't panic when team doesn't exist
    let result = show_team_conversation("nonexistent-team", Some(10), false);
    // Might fail or succeed depending on filesystem, just ensuring no panic
    let _ = result;
}
