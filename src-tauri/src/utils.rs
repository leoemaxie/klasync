use uuid::Uuid;

pub fn short_code() -> String {
    Uuid::new_v4().simple().to_string()[..6].to_uppercase()
}
