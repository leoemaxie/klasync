use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash(value: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(value.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

pub fn verify(value: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .ok()
        .and_then(|parsed| {
            Argon2::default()
                .verify_password(value.as_bytes(), &parsed)
                .ok()
        })
        .is_some()
}

pub async fn hash_async(value: String) -> Result<String, argon2::password_hash::Error> {
    tokio::task::spawn_blocking(move || hash(&value))
        .await
        .map_err(|_| argon2::password_hash::Error::Password)
        .and_then(|res| res)
}

pub async fn verify_async(value: String, hash_str: String) -> bool {
    tokio::task::spawn_blocking(move || verify(&value, &hash_str))
        .await
        .unwrap_or_else(|error| {
            tracing::error!(%error, "password verification blocking task panicked");
            false
        })
}
