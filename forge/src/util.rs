//! Small shared helpers used by both the LLM client and the GitHub client.

/// Truncate a string to at most `n` chars, on character boundaries.
pub fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// Best-effort parse of the HTTP `Retry-After` header as whole seconds.
/// Only the delta-seconds form is honored, not the HTTP-date form.
pub fn retry_after_secs(resp: &reqwest::Response) -> Option<u64> {
    resp.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_on_char_boundary() {
        assert_eq!(truncate("héllo", 2), "hé");
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("", 3), "");
    }
}
