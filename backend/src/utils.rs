use uuid::Uuid;

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
