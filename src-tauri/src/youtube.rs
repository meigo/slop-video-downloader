//! YouTube helpers — re-export multi-source module for back-compat.
//! Prefer `crate::source` for new code.

pub use crate::source::{
    is_supported_url, is_youtube_url, normalize_source_url, normalize_youtube_url, source_family,
    SourceFamily, UNSUPPORTED_SITE_MESSAGE,
};
