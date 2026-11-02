//! `file://` URIs as paths.

use std::path::PathBuf;

/// `None` for anything but a `file://` URI with valid escapes.
pub fn local_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let path = path.strip_prefix("localhost").unwrap_or(path);
    unescape(path).map(PathBuf::from)
}

fn unescape(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let high = (rest.next()? as char).to_digit(16)?;
            let low = (rest.next()? as char).to_digit(16)?;
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
#[path = "uri_test.rs"]
mod tests;
