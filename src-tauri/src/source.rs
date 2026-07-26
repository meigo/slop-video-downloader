//! Supported source URL allowlist (YouTube, Vimeo, X). Matches frontend `src/lib/source.ts`.

pub const UNSUPPORTED_SITE_MESSAGE: &str =
    "Unsupported site in this version. Supported: YouTube, Vimeo, X (Twitter) — public videos; X/Vimeo may need Chrome login.";

const VIDEO_ID_LEN: usize = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFamily {
    Youtube,
    Vimeo,
    X,
}

fn is_valid_youtube_id(id: &str) -> bool {
    id.len() == VIDEO_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_valid_vimeo_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_digit())
}

fn is_valid_x_status_id(id: &str) -> bool {
    let len = id.len();
    (5..=30).contains(&len) && id.chars().all(|c| c.is_ascii_digit())
}

fn is_youtube_host(host: &str) -> bool {
    matches!(
        host,
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "youtu.be" | "www.youtu.be"
    )
}

fn is_vimeo_host(host: &str) -> bool {
    matches!(host, "vimeo.com" | "www.vimeo.com" | "player.vimeo.com")
}

fn is_x_host(host: &str) -> bool {
    matches!(
        host,
        "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com"
    )
}

/// Parse http(s) URL into (host_lower, path, query).
fn split_http_url(url: &str) -> Option<(String, &str, &str)> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    let (host_part, after_host) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };

    let host = host_part
        .split(':')
        .next()
        .unwrap_or(host_part)
        .to_ascii_lowercase();

    let (path_and_query, _fragment) = match after_host.split_once('#') {
        Some((before, _)) => (before, ""),
        None => (after_host, ""),
    };

    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_query, ""),
    };

    Some((host, path, query))
}

fn extract_youtube_id(host: &str, path: &str, query: &str) -> Option<String> {
    if host == "youtu.be" || host == "www.youtu.be" {
        let id = path
            .trim_start_matches('/')
            .split('/')
            .find(|s| !s.is_empty())
            .unwrap_or("");
        return if is_valid_youtube_id(id) {
            Some(id.to_string())
        } else {
            None
        };
    }

    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("v=") {
            if is_valid_youtube_id(v) {
                return Some(v.to_string());
            }
        }
    }

    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() >= 2 && parts[0] == "shorts" && is_valid_youtube_id(parts[1]) {
        return Some(parts[1].to_string());
    }

    None
}

fn extract_vimeo_id(path: &str) -> Option<String> {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    if parts[0] == "video" && parts.len() >= 2 && is_valid_vimeo_id(parts[1]) {
        return Some(parts[1].to_string());
    }
    if is_valid_vimeo_id(parts[0]) {
        return Some(parts[0].to_string());
    }
    None
}

fn extract_x_status_id(path: &str) -> Option<String> {
    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == "status" {
            if let Some(id) = parts.get(i + 1) {
                if is_valid_x_status_id(id) {
                    return Some((*id).to_string());
                }
            }
        }
    }
    None
}

pub fn source_family(url: &str) -> Option<SourceFamily> {
    let (host, _, _) = split_http_url(url)?;
    if is_youtube_host(&host) {
        Some(SourceFamily::Youtube)
    } else if is_vimeo_host(&host) {
        Some(SourceFamily::Vimeo)
    } else if is_x_host(&host) {
        Some(SourceFamily::X)
    } else {
        None
    }
}

pub fn is_supported_url(url: &str) -> bool {
    normalize_source_url(url).is_some()
}

/// Canonical URL for yt-dlp, or None if unsupported.
pub fn normalize_source_url(url: &str) -> Option<String> {
    let (host, path, query) = split_http_url(url)?;

    if is_youtube_host(&host) {
        let id = extract_youtube_id(&host, path, query)?;
        return Some(format!("https://www.youtube.com/watch?v={id}"));
    }

    if is_vimeo_host(&host) {
        let id = extract_vimeo_id(path)?;
        return Some(format!("https://vimeo.com/{id}"));
    }

    if is_x_host(&host) {
        let id = extract_x_status_id(path)?;
        return Some(format!("https://x.com/i/status/{id}"));
    }

    None
}

// --- YouTube-only helpers (back-compat for existing tests / call sites) ---

pub fn is_youtube_url(url: &str) -> bool {
    matches!(source_family(url), Some(SourceFamily::Youtube)) && normalize_source_url(url).is_some()
}

pub fn normalize_youtube_url(url: &str) -> Option<String> {
    if !matches!(source_family(url), Some(SourceFamily::Youtube)) {
        return None;
    }
    normalize_source_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_youtube() {
        assert!(is_supported_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_supported_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_supported_url("https://www.youtube.com/shorts/dQw4w9WgXcQ"));
    }

    #[test]
    fn accepts_vimeo() {
        assert!(is_supported_url("https://vimeo.com/123456789"));
        assert!(is_supported_url("https://player.vimeo.com/video/123456789"));
        assert_eq!(
            normalize_source_url("https://player.vimeo.com/video/123456789").as_deref(),
            Some("https://vimeo.com/123456789")
        );
    }

    #[test]
    fn accepts_x_twitter() {
        assert!(is_supported_url(
            "https://x.com/someuser/status/1234567890123456789"
        ));
        assert!(is_supported_url(
            "https://twitter.com/someuser/status/1234567890123456789"
        ));
        assert!(is_supported_url("https://x.com/i/status/1234567890123456789"));
        assert!(is_supported_url(
            "https://mobile.twitter.com/i/web/status/1234567890123456789"
        ));
        assert_eq!(
            normalize_source_url("https://twitter.com/foo/status/1234567890123456789").as_deref(),
            Some("https://x.com/i/status/1234567890123456789")
        );
        // Host is X, but status id too short → not supported
        assert_eq!(normalize_source_url("https://x.com/a/status/1"), None);
        assert_eq!(
            source_family("https://x.com/a/status/12345678901"),
            Some(SourceFamily::X)
        );
    }

    #[test]
    fn rejects_instagram_and_garbage() {
        assert!(!is_supported_url("https://instagram.com/p/abc"));
        assert!(!is_supported_url("https://example.com"));
        assert!(!is_supported_url("not a url"));
        assert!(!is_supported_url("https://vimeo.com/"));
        assert!(!is_supported_url("https://x.com/home"));
    }

    #[test]
    fn normalizes_youtu_be() {
        assert_eq!(
            normalize_source_url("https://youtu.be/dQw4w9WgXcQ").as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }

    #[test]
    fn youtube_compat_rejects_vimeo() {
        assert!(!is_youtube_url("https://vimeo.com/123"));
        assert_eq!(normalize_youtube_url("https://vimeo.com/123"), None);
    }
}
