use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRng, rngs::SysRng};
use sha2::{Digest, Sha256};

/// Marks a string as a tally token, so a leaked one is easy to recognize.
const PREFIX: &str = "tly_";

/// Returns a new random token: `tly_` followed by 32 bytes from the OS
/// random number generator, encoded as base64url without padding.
///
/// # Errors
/// Returns an error if the OS random number generator fails.
pub fn generate() -> Result<String> {
    let mut bytes = [0u8; 32];
    SysRng
        .try_fill_bytes(&mut bytes)
        .with_context(|| "failed to read from the OS random number generator")?;

    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    Ok(format!("{PREFIX}{encoded}"))
}

/// Returns the SHA-256 hash of `token`. Only this hash is stored in the database.
pub fn hash(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_returns_prefixed_base64url_of_32_bytes() {
        let token = generate().expect("token should be generated");

        let encoded = token
            .strip_prefix("tly_")
            .expect("token should start with tly_");
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .expect("token should be base64url without padding");
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn generate_returns_a_different_token_each_time() {
        let first = generate().expect("first token should be generated");
        let second = generate().expect("second token should be generated");

        assert_ne!(first, second);
    }

    #[test]
    fn hash_is_sha256_of_token() {
        let hash = hash("abc");

        // SHA-256 of "abc" (FIPS 180-2 test vector), encoded as base64url
        assert_eq!(
            URL_SAFE_NO_PAD.encode(hash),
            "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
        );
    }
}
