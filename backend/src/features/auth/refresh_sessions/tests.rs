#[cfg(test)]
mod tests {
    use crate::jwt::{Claims, TokenType, REFRESH_TOKEN_TTL_SECONDS};
    use chrono::Utc;

    #[test]
    fn refresh_token_has_correct_ttl() {
        let now = Utc::now().timestamp() as usize;
        let claims = Claims::for_refresh("user-id", "jti-123".to_string(), now);

        assert_eq!(claims.typ, Some(TokenType::Refresh));
        assert_eq!(claims.jti, Some("jti-123".to_string()));
        assert_eq!(claims.exp - claims.iat, REFRESH_TOKEN_TTL_SECONDS);
    }

    #[test]
    fn access_token_with_jti_has_correct_properties() {
        let now = Utc::now().timestamp() as usize;
        let jti = "session-123".to_string();
        let claims = Claims::for_access_with_jti("user-id", jti.clone(), now);

        assert_eq!(claims.typ, Some(TokenType::Access));
        assert_eq!(claims.jti, Some(jti));
        assert_eq!(claims.sub, "user-id");
    }

    #[test]
    fn refresh_and_access_tokens_are_distinguishable() {
        let now = Utc::now().timestamp() as usize;
        let access = Claims::for_user("user-id", now);
        let refresh = Claims::for_refresh("user-id", "jti-123".to_string(), now);

        assert_eq!(access.typ, Some(TokenType::Access));
        assert_eq!(refresh.typ, Some(TokenType::Refresh));
        assert_ne!(access.exp, refresh.exp);
    }
}

