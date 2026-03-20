/// strip query parameters from a url to avoid logging credentials
///
/// returns everything before `?`, or the full string if no query is present
pub(crate) fn redact_url(url: &str) -> &str {
    match url.split_once('?') {
        Some((base, _)) => base,
        None => url,
    }
}
