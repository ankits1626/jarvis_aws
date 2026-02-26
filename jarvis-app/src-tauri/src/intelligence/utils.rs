// Shared utilities for intelligence providers

/// Snap a byte index down to the nearest valid UTF-8 char boundary.
pub fn snap_to_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Split content into chunks at paragraph/line boundaries, each <= max_chars.
/// All slice boundaries are snapped to valid UTF-8 char boundaries.
pub fn split_content(content: &str, max_chars: usize) -> Vec<&str> {
    if content.len() <= max_chars {
        return vec![content];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < content.len() {
        if start + max_chars >= content.len() {
            chunks.push(&content[start..]);
            break;
        }

        let end = snap_to_char_boundary(content, start + max_chars);

        // Try to break at paragraph boundary (double newline) within last 500 chars
        let search_start = snap_to_char_boundary(content, if end > start + 500 { end - 500 } else { start });
        let break_pos = content[search_start..end]
            .rfind("\n\n")
            .map(|pos| search_start + pos + 2)
            .or_else(|| {
                content[search_start..end]
                    .rfind('\n')
                    .map(|pos| search_start + pos + 1)
            })
            .or_else(|| {
                content[search_start..end]
                    .rfind(' ')
                    .map(|pos| search_start + pos + 1)
            })
            .unwrap_or(end);

        chunks.push(&content[start..break_pos]);
        start = break_pos;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snap_to_char_boundary() {
        let s = "Hello 世界";
        assert_eq!(snap_to_char_boundary(s, 0), 0);
        assert_eq!(snap_to_char_boundary(s, 6), 6);
        // Index 7 is in the middle of '世' (3 bytes)
        assert_eq!(snap_to_char_boundary(s, 7), 6);
        assert_eq!(snap_to_char_boundary(s, 100), s.len());
    }

    #[test]
    fn test_split_content_small() {
        let content = "Hello world";
        let chunks = split_content(content, 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], content);
    }

    #[test]
    fn test_split_content_paragraph_boundary() {
        let content = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let chunks = split_content(content, 30);
        assert!(chunks.len() >= 2);
        // Verify concatenation equals original
        assert_eq!(chunks.join(""), content);
    }

    #[test]
    fn test_split_content_utf8_safety() {
        let content = "Hello 世界 ".repeat(100);
        let chunks = split_content(&content, 50);
        // Verify all chunks are valid UTF-8
        for chunk in &chunks {
            assert!(std::str::from_utf8(chunk.as_bytes()).is_ok());
        }
        // Verify concatenation equals original
        assert_eq!(chunks.join(""), content);
    }

    #[test]
    fn test_split_content_preserves_information() {
        let content = "a".repeat(10000);
        let chunks = split_content(&content, 1000);
        assert_eq!(chunks.join(""), content);
    }
}
