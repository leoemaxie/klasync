use uuid::Uuid;

pub fn short_code() -> String {
    Uuid::now_v7().simple().to_string()[..8].to_uppercase()
}
