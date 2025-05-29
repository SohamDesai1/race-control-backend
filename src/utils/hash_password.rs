use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Algorithm, Argon2, Params, Version,
};

pub fn hash_password(password: &str) -> Result<String, String> {
    // Configure Argon2 parameters for security
    let params =
        Params::new(15, 2, 1, None).map_err(|e| format!("Invalid Argon2 parameters: {}", e))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    // Generate cryptographically secure salt
    let salt = SaltString::generate(&mut OsRng);

    // Hash password and handle potential errors properly
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Hashing failed: {}", e))?
        .to_string();

    Ok(password_hash)
}
