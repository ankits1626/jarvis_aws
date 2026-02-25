#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::super::accessibility::AccessibilityReader;

    #[test]
    fn test_check_permission_returns_boolean() {
        // This test verifies that check_permission() returns a boolean without panicking
        let result = AccessibilityReader::check_permission();
        
        // The result should be either true or false, not panic
        assert!(result == true || result == false);
    }
}
