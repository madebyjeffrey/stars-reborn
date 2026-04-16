pub mod handler;
pub mod model;
pub mod routes;

use sea_orm::{DatabaseConnection, EntityTrait};
use sea_orm::ActiveModelTrait;
use sha2::{Digest, Sha256};

pub fn hash_api_token(raw_token: &str, secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(b":");
    hasher.update(raw_token.as_bytes());
    hex::encode(hasher.finalize())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub async fn assert_all_tokens_hashed(db: &DatabaseConnection) -> anyhow::Result<()> {
    let invalid_count = count_non_hashed_tokens(db).await?;

    if invalid_count > 0 {
        anyhow::bail!(
            "Found {} API token(s) not stored as SHA-256 hashes; refusing to start",
            invalid_count
        );
    }

    Ok(())
}

pub async fn count_non_hashed_tokens(db: &DatabaseConnection) -> anyhow::Result<usize> {
    let tokens = model::Entity::find().all(db).await?;
    Ok(tokens
        .iter()
        .filter(|token| !is_sha256_hex(&token.token))
        .count())
}

pub async fn purge_non_hashed_tokens(db: &DatabaseConnection) -> anyhow::Result<usize> {
    let tokens = model::Entity::find().all(db).await?;
    let invalid_tokens: Vec<model::Model> = tokens
        .into_iter()
        .filter(|token| !is_sha256_hex(&token.token))
        .collect();

    let removed = invalid_tokens.len();
    for token in invalid_tokens {
        let active: model::ActiveModel = token.into();
        active.delete(db).await?;
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::{hash_api_token, is_sha256_hex};

    #[test]
    fn hash_is_deterministic_for_same_inputs() {
        let secret = "server-secret";
        let token = "raw-api-token";

        let first = hash_api_token(token, secret);
        let second = hash_api_token(token, secret);

        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn hash_changes_when_token_changes() {
        let secret = "server-secret";

        let first = hash_api_token("token-a", secret);
        let second = hash_api_token("token-b", secret);

        assert_ne!(first, second);
    }

    #[test]
    fn hash_changes_when_secret_changes() {
        let token = "raw-api-token";

        let first = hash_api_token(token, "secret-a");
        let second = hash_api_token(token, "secret-b");

        assert_ne!(first, second);
    }

    #[test]
    fn accepts_sha256_hex_format() {
        let digest = hash_api_token("token", "secret");
        assert!(is_sha256_hex(&digest));
    }

    #[test]
    fn rejects_non_sha256_hex_format() {
        assert!(!is_sha256_hex("not-a-digest"));
        assert!(!is_sha256_hex(&"a".repeat(63)));
        assert!(!is_sha256_hex(&"g".repeat(64)));
    }

}
