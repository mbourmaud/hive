pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet_returns_correct_message() {
        assert_eq!(greet("World"), "Hello, World!");
    }
}
