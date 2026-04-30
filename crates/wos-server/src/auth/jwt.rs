use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AuthContext, AuthError, AuthProvider, AuthResult, AuthUser, TokenPair};
use crate::storage::{SessionRow, StorageError, StorageHandle};

fn ae(e: StorageError) -> AuthError {
    AuthError::Other(e.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    jti: String,
    role: String,
    name: String,
    email: String,
    /// Omitted in tokens issued before this field existed.
    #[serde(default)]
    avatar: Option<String>,
    /// Must match `users.auth_epoch`; bumped on logout.
    #[serde(default)]
    auth_epoch: i64,
    exp: i64,
    iat: i64,
    /// `access` or `refresh`.
    kind: String,
}

pub struct JwtAuth {
    encode_key: EncodingKey,
    decode_key: DecodingKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
    storage: StorageHandle,
}

impl JwtAuth {
    pub fn new(
        secret: &[u8],
        access_ttl_secs: i64,
        refresh_ttl_secs: i64,
        storage: StorageHandle,
    ) -> Self {
        Self {
            encode_key: EncodingKey::from_secret(secret),
            decode_key: DecodingKey::from_secret(secret),
            access_ttl: Duration::seconds(access_ttl_secs.max(60)),
            refresh_ttl: Duration::seconds(refresh_ttl_secs.max(300)),
            storage,
        }
    }

    fn issue(
        &self,
        user: &AuthUser,
        kind: &str,
        ttl: Duration,
        jti: &str,
        auth_epoch: i64,
    ) -> AuthResult<(String, chrono::DateTime<Utc>)> {
        let now = Utc::now();
        let exp = now + ttl;
        let claims = Claims {
            sub: user.id.clone(),
            jti: jti.to_string(),
            role: user.role.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            avatar: user.avatar.clone(),
            auth_epoch,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            kind: kind.to_string(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encode_key)
            .map_err(|e| AuthError::Other(e.to_string()))?;
        Ok((token, exp))
    }

    fn decode_claims(&self, token: &str) -> AuthResult<Claims> {
        let validation = Validation::new(Algorithm::HS256);
        let data = decode::<Claims>(token, &self.decode_key, &validation)
            .map_err(|_| AuthError::InvalidToken)?;
        Ok(data.claims)
    }
}

#[async_trait]
impl AuthProvider for JwtAuth {
    async fn login(&self, email: &str, password: &str) -> AuthResult<TokenPair> {
        let user_row = self
            .storage
            .get_user_by_email(email)
            .await
            .map_err(ae)?
            .ok_or(AuthError::InvalidCredentials)?;

        let parsed = PasswordHash::new(&user_row.password_hash)
            .map_err(|_| AuthError::Other("stored password hash malformed".into()))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .map_err(|_| AuthError::InvalidCredentials)?;

        let user = AuthUser {
            id: user_row.id.clone(),
            name: user_row.name.clone(),
            email: user_row.email.clone(),
            role: user_row.role.clone(),
            avatar: user_row.avatar.clone(),
        };

        let epoch = user_row.auth_epoch;
        let access_jti = Uuid::now_v7().to_string();
        let refresh_jti = Uuid::now_v7().to_string();
        let (access_token, access_exp) =
            self.issue(&user, "access", self.access_ttl, &access_jti, epoch)?;
        let (refresh_token, refresh_exp) =
            self.issue(&user, "refresh", self.refresh_ttl, &refresh_jti, epoch)?;

        for (jti, exp) in [(&access_jti, access_exp), (&refresh_jti, refresh_exp)] {
            self.storage
                .upsert_session(&SessionRow {
                    jti: jti.clone(),
                    user_id: user_row.id.clone(),
                    expires_at: exp,
                    revoked: false,
                })
                .await
                .map_err(ae)?;
        }

        Ok(TokenPair {
            access_token,
            refresh_token,
            access_expires_at: access_exp,
            refresh_expires_at: refresh_exp,
            user,
        })
    }

    async fn refresh(&self, refresh_token: &str) -> AuthResult<TokenPair> {
        let claims = self.decode_claims(refresh_token)?;
        if claims.kind != "refresh" {
            return Err(AuthError::InvalidToken);
        }
        let user_row = self
            .storage
            .get_user(&claims.sub)
            .await
            .map_err(ae)?
            .ok_or(AuthError::InvalidToken)?;
        if claims.auth_epoch != user_row.auth_epoch {
            return Err(AuthError::Revoked);
        }
        if !self.storage.session_is_valid(&claims.jti).await.map_err(ae)? {
            return Err(AuthError::Revoked);
        }
        self.storage.revoke_session(&claims.jti).await.map_err(ae)?;

        let user = AuthUser {
            id: user_row.id.clone(),
            name: user_row.name.clone(),
            email: user_row.email.clone(),
            role: user_row.role.clone(),
            avatar: user_row.avatar.clone(),
        };

        let access_jti = Uuid::now_v7().to_string();
        let refresh_jti = Uuid::now_v7().to_string();
        let epoch = user_row.auth_epoch;
        let (access_token, access_exp) =
            self.issue(&user, "access", self.access_ttl, &access_jti, epoch)?;
        let (refresh_token_new, refresh_exp) =
            self.issue(&user, "refresh", self.refresh_ttl, &refresh_jti, epoch)?;

        for (jti, exp) in [(&access_jti, access_exp), (&refresh_jti, refresh_exp)] {
            self.storage
                .upsert_session(&SessionRow {
                    jti: jti.clone(),
                    user_id: user.id.clone(),
                    expires_at: exp,
                    revoked: false,
                })
                .await
                .map_err(ae)?;
        }

        Ok(TokenPair {
            access_token,
            refresh_token: refresh_token_new,
            access_expires_at: access_exp,
            refresh_expires_at: refresh_exp,
            user,
        })
    }

    async fn logout(&self, access_token: &str) -> AuthResult<()> {
        let claims = self.decode_claims(access_token)?;
        if claims.kind != "access" {
            return Err(AuthError::InvalidToken);
        }
        self.storage.bump_user_auth_epoch(&claims.sub).await.map_err(ae)?;
        self.storage.revoke_sessions_for_user(&claims.sub).await.map_err(ae)?;
        Ok(())
    }

    async fn verify(&self, access_token: &str) -> AuthResult<AuthContext> {
        let claims = self.decode_claims(access_token)?;
        if claims.kind != "access" {
            return Err(AuthError::InvalidToken);
        }
        let user_row = self
            .storage
            .get_user(&claims.sub)
            .await
            .map_err(ae)?
            .ok_or(AuthError::InvalidToken)?;
        if claims.auth_epoch != user_row.auth_epoch {
            return Err(AuthError::Revoked);
        }
        if !self.storage.session_is_valid(&claims.jti).await.map_err(ae)? {
            return Err(AuthError::Revoked);
        }
        Ok(AuthContext {
            user: AuthUser {
                id: user_row.id,
                name: user_row.name,
                email: user_row.email,
                role: user_row.role,
                avatar: user_row.avatar,
            },
            jti: claims.jti,
            access_token: None,
        })
    }
}
