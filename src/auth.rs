use crate::config::JwtConfig;
use crate::models::Claims;
use anyhow::{anyhow, Result};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use sqlx::PgPool;
use uuid::Uuid;

pub struct JwtAuth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    expiration_hours: i64,
}

impl JwtAuth {
    pub fn new(config: &JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());
        let validation = Validation::default();

        Self {
            encoding_key,
            decoding_key,
            validation,
            expiration_hours: config.expiration_hours,
        }
    }

    /// Generate a new JWT token for a user
    pub fn generate_token(&self, user_id: Uuid, email: &str, role: &str) -> Result<String> {
        let now = Utc::now().timestamp();
        let exp = now + (self.expiration_hours * 3600);

        let claims = Claims {
            sub: email.to_string(),
            user_id,
            role: role.to_string(),
            exp,
            iat: now,
            jti: Uuid::new_v4(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Token generation failed: {}", e))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map(|data| data.claims)
            .map_err(|e| anyhow!("Token validation failed: {}", e))
    }

    /// Check if a token is revoked (requires database check)
    pub async fn is_token_revoked(&self, jti: Uuid, pool: &PgPool) -> Result<bool> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM revoked_tokens WHERE jti = $1 AND expires_at > now())"
        )
        .bind(jti)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Revoke a token (for logout)
    pub async fn revoke_token(&self, claims: &Claims, pool: &PgPool) -> Result<()> {
        sqlx::query(
            "INSERT INTO revoked_tokens (jti, user_id, expires_at) VALUES ($1, $2, to_timestamp($3))"
        )
        .bind(claims.jti)
        .bind(claims.user_id)
        .bind(claims.exp)
        .execute(pool)
        .await?;

        Ok(())
    }
}

/// Extract Bearer token from Authorization header
pub fn extract_bearer_token(auth_header: Option<&str>) -> Result<String> {
    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            Ok(header.trim_start_matches("Bearer ").to_string())
        }
        _ => Err(anyhow!("Missing or invalid Authorization header")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::JwtConfig;

    #[test]
    fn test_token_generation_and_validation() {
        let config = JwtConfig {
            secret: "test_secret_key_minimum_32_chars_long_for_security".to_string(),
            expiration_hours: 24,
            refresh_token_days: 7,
        };

        let auth = JwtAuth::new(&config);
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let role = "viewer";

        let token = auth.generate_token(user_id, email, role).expect("Token generation failed");
        let claims = auth.validate_token(&token).expect("Token validation failed");

        assert_eq!(claims.sub, email);
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig {
            secret: "test_secret_key_minimum_32_chars_long_for_security".to_string(),
            expiration_hours: 24,
            refresh_token_days: 7,
        };

        let auth = JwtAuth::new(&config);
        let result = auth.validate_token("invalid.token.here");

        assert!(result.is_err());
    }
}
