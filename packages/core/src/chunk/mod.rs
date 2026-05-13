//! Plain text / Markdown MVP chunking: paragraph splits first, then sliding windows.

/// Target maximum chunk length in UTF-8 scalar characters.
pub const DEFAULT_CHUNK_MAX_CHARS: usize = 768;

/// Overlap between consecutive windows when sliding.
pub const DEFAULT_CHUNK_OVERLAP_CHARS: usize = 96;

/// MVP strategy: split on blank lines (`\n\n`); oversized paragraphs use fixed-size windows with overlap.
pub fn chunk_plain_text_markdown_mvp(text: &str) -> Vec<String> {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    for para in trimmed.split("\n\n") {
        let p = para.trim();
        if p.is_empty() {
            continue;
        }
        let chars: Vec<char> = p.chars().collect();
        if chars.len() <= DEFAULT_CHUNK_MAX_CHARS {
            chunks.push(chars.iter().collect());
            continue;
        }
        let mut start = 0usize;
        while start < chars.len() {
            let end = (start + DEFAULT_CHUNK_MAX_CHARS).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk.trim().to_string());
            if end >= chars.len() {
                break;
            }
            let step = DEFAULT_CHUNK_MAX_CHARS
                .saturating_sub(DEFAULT_CHUNK_OVERLAP_CHARS)
                .max(1);
            start += step;
        }
    }

    if chunks.is_empty() {
        chunks.push(trimmed.to_string());
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_no_chunks() {
        assert!(chunk_plain_text_markdown_mvp("").is_empty());
        assert!(chunk_plain_text_markdown_mvp("  \n\t").is_empty());
    }

    #[test]
    fn short_paragraphs_become_separate_chunks() {
        let v = chunk_plain_text_markdown_mvp("hello\n\nworld");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0], "hello");
        assert_eq!(v[1], "world");
    }

    /// 超长段落含多字节字符：按字节滑窗曾在 `&str` 非字符边界切片时 panic；此处回归为按标量字符滑窗。
    #[test]
    fn sliding_windows_do_not_panic_on_multibyte_char_boundaries() {
        let mut s = String::new();
        s.extend(std::iter::repeat_n('a', 767));
        s.push('世');
        s.extend(std::iter::repeat_n('b', 400));
        let out = std::panic::catch_unwind(|| chunk_plain_text_markdown_mvp(&s));
        assert!(out.is_ok(), "expected chunking not to panic");
        let v = out.unwrap();
        assert!(!v.is_empty());
        for piece in &v {
            assert!(
                piece.chars().count() <= DEFAULT_CHUNK_MAX_CHARS,
                "chunk too long: {} scalars",
                piece.chars().count()
            );
        }
    }
}
