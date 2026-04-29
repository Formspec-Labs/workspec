use async_trait::async_trait;
use chrono::{Duration, Utc};
use wos_server_ports::auth::{AuthContext, AuthProvider, AuthResult, AuthUser, TokenPair};

pub struct MockAuth {
    user: AuthUser,
    jti: String,
}

impl Default for MockAuth {
    fn default() -> Self {
        Self {
            user: AuthUser {
                id: "user-jane-doe".into(),
                name: "Jane Doe".into(),
                email: "jane.doe@example.gov".into(),
                role: "Supervisor".into(),
                avatar: None,
            },
            jti: "mock-jti".into(),
        }
    }
}

impl MockAuth {
    pub fn with_user(user: AuthUser) -> Self {
        Self {
            user,
            jti: "mock-jti".into(),
        }
    }
}

#[async_trait]
impl AuthProvider for MockAuth {
    async fn login(&self, _email: &str, _password: &str) -> AuthResult<TokenPair> {
        let now = Utc::now();
        Ok(TokenPair {
            access_token: "mock-access".into(),
            refresh_token: "mock-refresh".into(),
            access_expires_at: now + Duration::hours(1),
            refresh_expires_at: now + Duration::days(7),
            user: self.user.clone(),
        })
    }

    async fn refresh(&self, _refresh_token: &str) -> AuthResult<TokenPair> {
        self.login("", "").await
    }

    async fn logout(&self, _access_token: &str) -> AuthResult<()> {
        Ok(())
    }

    async fn verify(&self, _access_token: &str) -> AuthResult<AuthContext> {
        Ok(AuthContext {
            user: self.user.clone(),
            jti: self.jti.clone(),
            access_token: None,
        })
    }
}
