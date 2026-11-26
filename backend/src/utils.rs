use uuid::Uuid;

/// Lowercase hex of a byte slice (digest output, same text `{:x}` gave for sha2 0.10).
pub fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    use std::fmt::Write;
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
    }
    out
}

pub fn short_code() -> String {
    const CHARSET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let bytes = Uuid::new_v4().into_bytes();
    let mut code = String::with_capacity(8);
    for &byte in &bytes[..8] {
        let index = (byte & 31) as usize;
        code.push(CHARSET[index] as char);
    }
    code
}
