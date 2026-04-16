use chrono::Utc;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};

pub const ACCESS_TOKEN_ALGORITHM: Algorithm = Algorithm::HS256;
pub const ACCESS_TOKEN_TTL_SECONDS: usize = 60 * 60 * 24 * 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

impl Claims {
    pub fn for_user(user_id: impl Into<String>, issued_at: usize) -> Self {
        Self {
            sub: user_id.into(),
            exp: issued_at + ACCESS_TOKEN_TTL_SECONDS,
            iat: issued_at,
        }
    }

    pub fn for_api_token(user_id: impl Into<String>, issued_at: usize) -> Self {
        Self {
            sub: user_id.into(),
            exp: usize::MAX,
            iat: issued_at,
        }
    }
}

pub fn issue_access_token(
    user_id: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::for_user(user_id, Utc::now().timestamp() as usize);

    encode(
        &Header::new(ACCESS_TOKEN_ALGORITHM),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn decode_access_token(
    token: &str,
    secret: &str,
) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(ACCESS_TOKEN_ALGORITHM),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        decode_access_token, issue_access_token, Claims, ACCESS_TOKEN_ALGORITHM,
        ACCESS_TOKEN_TTL_SECONDS,
    };

    #[test]
    fn claims_for_user_sets_expected_expiry_window() {
        let claims = Claims::for_user("pilot-id", 1_700_000_000);

        assert_eq!(claims.sub, "pilot-id");
        assert_eq!(claims.iat, 1_700_000_000);
        assert_eq!(claims.exp, 1_700_000_000 + ACCESS_TOKEN_TTL_SECONDS);
    }

    #[test]
    fn claims_for_api_token_use_non_expiring_sentinel() {
        let claims = Claims::for_api_token("pilot-id", 1_700_000_000);

        assert_eq!(claims.sub, "pilot-id");
        assert_eq!(claims.iat, 1_700_000_000);
        assert_eq!(claims.exp, usize::MAX);
    }

    #[test]
    fn issue_and_decode_access_token_round_trip_uses_shared_contract() {
        let token = issue_access_token("pilot-id", "super-secret")
            .expect("token should encode");
        let decoded =
            decode_access_token(&token, "super-secret").expect("token should decode");

        assert_eq!(decoded.header.alg, ACCESS_TOKEN_ALGORITHM);
        assert_eq!(decoded.claims.sub, "pilot-id");
        assert!(decoded.claims.exp > decoded.claims.iat);
    }
}
