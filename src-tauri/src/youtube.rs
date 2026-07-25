//! YouTube URL validation and normalization (matches frontend `src/lib/youtube.ts`).

const VIDEO_ID_LEN: usize = 11;

fn is_valid_video_id(id: &str) -> bool {
    id.len() == VIDEO_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn is_allowed_host(host: &str) -> bool {
    matches!(
        host,
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "youtu.be" | "www.youtu.be"
    )
}

/// Extract an 11-char video id from a YouTube URL string, if valid.
fn parse_youtube_url(url: &str) -> Option<String> {
    let url = url.trim();
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;

    // host[/path][?query][#fragment]
    let (host_part, after_host) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };

    // Drop optional port
    let host = host_part
        .split(':')
        .next()
        .unwrap_or(host_part)
        .to_ascii_lowercase();

    if !is_allowed_host(&host) {
        return None;
    }

    let (path_and_maybe_query, _fragment) = match after_host.split_once('#') {
        Some((before, after)) => (before, after),
        None => (after_host, ""),
    };

    let (path, query) = match path_and_maybe_query.split_once('?') {
        Some((p, q)) => (p, q),
        None => (path_and_maybe_query, ""),
    };

    if host == "youtu.be" || host == "www.youtu.be" {
        let id = path
            .trim_start_matches('/')
            .split('/')
            .find(|s| !s.is_empty())
            .unwrap_or("");
        return if is_valid_video_id(id) {
            Some(id.to_string())
        } else {
            None
        };
    }

    // youtube.com / www / m — prefer v= query, then /shorts/ID
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("v=") {
            if is_valid_video_id(v) {
                return Some(v.to_string());
            }
        }
    }

    let parts: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() >= 2 && parts[0] == "shorts" && is_valid_video_id(parts[1]) {
        return Some(parts[1].to_string());
    }

    None
}

/// Returns true if `url` is a YouTube watch, short, or youtu.be link with a valid video id.
pub fn is_youtube_url(url: &str) -> bool {
    parse_youtube_url(url).is_some()
}

/// Canonical `https://www.youtube.com/watch?v=ID`, or `None` if not a valid YouTube URL.
pub fn normalize_youtube_url(url: &str) -> Option<String> {
    let id = parse_youtube_url(url)?;
    Some(format!("https://www.youtube.com/watch?v={id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_watch_and_short() {
        assert!(is_youtube_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(is_youtube_url("https://youtu.be/dQw4w9WgXcQ"));
        assert!(is_youtube_url("https://www.youtube.com/shorts/dQw4w9WgXcQ"));
    }

    #[test]
    fn rejects_vimeo() {
        assert!(!is_youtube_url("https://vimeo.com/123"));
        assert!(!is_youtube_url("not a url"));
    }

    #[test]
    fn normalizes_youtu_be() {
        assert_eq!(
            normalize_youtube_url("https://youtu.be/dQw4w9WgXcQ").as_deref(),
            Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ")
        );
    }

    #[test]
    fn normalize_rejects_non_youtube() {
        assert_eq!(normalize_youtube_url("https://example.com"), None);
    }
}
