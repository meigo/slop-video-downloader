//! Clip filename helpers (matches frontend `src/lib/filename.ts` + `formatFilenameTime`).

/// Filename-safe time token, e.g. `01m12s` or `01h01m01s` (whole seconds).
pub fn format_filename_time(secs: f64) -> String {
    let whole = secs.max(0.0).floor() as u64;
    let hours = whole / 3600;
    let minutes = (whole % 3600) / 60;
    let seconds = whole % 60;

    if hours > 0 {
        format!("{hours:02}h{minutes:02}m{seconds:02}s")
    } else {
        format!("{minutes:02}m{seconds:02}s")
    }
}

/// Strip path-unsafe chars, collapse whitespace. Empty result becomes `"clip"`.
pub fn sanitize_title(title: &str) -> String {
    // Replace / \ ? % * : | " < > and control chars with space (matches TS).
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>' => ' ',
            c if (c as u32) <= 0x1f || c == '\u{7f}' => ' ',
            c => c,
        })
        .collect();

    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        "clip".to_string()
    } else {
        collapsed
    }
}

/// `{sanitized}_{start}-{end}.mp4` using filename time tokens.
pub fn build_clip_filename(title: &str, start_secs: f64, end_secs: f64) -> String {
    let safe = sanitize_title(title);
    let start = format_filename_time(start_secs);
    let end = format_filename_time(end_secs);
    format!("{safe}_{start}-{end}.mp4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_filename_time_tokens() {
        assert_eq!(format_filename_time(72.0), "01m12s");
        assert_eq!(format_filename_time(3661.0), "01h01m01s");
    }

    #[test]
    fn sanitize_strips_unsafe_and_collapses() {
        assert_eq!(
            sanitize_title(r#"My Video: "Cool"/Thing?"#),
            "My Video Cool Thing"
        );
        assert_eq!(sanitize_title("  a   b  "), "a b");
    }

    #[test]
    fn build_clip_filename_format() {
        assert_eq!(
            build_clip_filename("Hello World", 72.0, 105.0),
            "Hello World_01m12s-01m45s.mp4"
        );
    }
}
