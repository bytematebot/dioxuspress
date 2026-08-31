use std::collections::HashMap;

/// Turns heading text into a URL fragment: "Hello, World!" -> "hello-world".
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_dash = true; // leading dashes are trimmed
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("section");
    }
    out
}

/// Hands out unique slugs within one page, appending `-1`, `-2`, ... on collision.
#[derive(Default)]
pub struct SlugAllocator {
    seen: HashMap<String, usize>,
}

impl SlugAllocator {
    pub fn allocate(&mut self, text: &str) -> String {
        let base = slugify(text);
        let count = self.seen.entry(base.clone()).or_insert(0);
        *count += 1;
        if *count == 1 {
            base
        } else {
            format!("{base}-{}", *count - 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_strips_punctuation_and_case() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("  spaced  out  "), "spaced-out");
        assert_eq!(slugify("!!!"), "section");
    }

    #[test]
    fn allocator_dedupes() {
        let mut alloc = SlugAllocator::default();
        assert_eq!(alloc.allocate("Example"), "example");
        assert_eq!(alloc.allocate("Example"), "example-1");
        assert_eq!(alloc.allocate("Example"), "example-2");
    }
}
