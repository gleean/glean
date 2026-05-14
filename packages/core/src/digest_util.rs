//! Lowercase hex encoding for digest `Output` values (sha2 / digest 0.11).

use std::fmt::Write;

pub(crate) fn digest_to_hex_lower(digest: impl AsRef<[u8]>) -> String {
    let bytes = digest.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(&mut out, "{:02x}", b);
    }
    out
}
