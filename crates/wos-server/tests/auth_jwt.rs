//! Auth provider behaviour: login issues a valid token, verify round-trips
//! it, revocation invalidates, refresh rotates jtis.

use std::sync::Arc;

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use chrono::Utc;
use rand::rngs::OsRng;
use wos_server::auth::{AuthProvider, JwtAuth};
use wos_server::storage::{SqliteStorage, Storage, UserRow};

async fn seeded_store_and_auth() -> (Arc<SqliteStorage>, JwtAuth) {
    let store = Arc::new(
        SqliteStorage::connect("sqlite::memory:?cache=shared")
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(b"wos-dev", &salt)
        .unwrap()
        .to_string();
    store
        .upsert_user(&UserRow {
            id: "u1".into(),
            email: "user@example.com".into(),
            name: "User One".into(),
            role: "Supervisor".into(),
            password_hash: hash,
            avatar: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let handle: wos_server::storage::StorageHandle = store.clone();
    let jwt = JwtAuth::new(b"test-secret-not-for-prod", 900, 7 * 24 * 3600, handle);
    (store, jwt)
}

#[tokio::test]
async fn login_issues_tokens_that_verify() {
    let (_store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    assert_eq!(pair.user.role, "Supervisor");

    let ctx = auth.verify(&pair.access_token).await.unwrap();
    assert_eq!(ctx.user.email, "user@example.com");
}

#[tokio::test]
async fn invalid_credentials_fail() {
    let (_store, auth) = seeded_store_and_auth().await;
    assert!(auth.login("user@example.com", "wrong").await.is_err());
    assert!(auth.login("nobody@example.com", "wos-dev").await.is_err());
}

#[tokio::test]
async fn logout_revokes_the_access_token() {
    let (_store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    // Verify first — this sanity-checks the happy path.
    assert!(auth.verify(&pair.access_token).await.is_ok());

    // The server's /auth/logout handler revokes via the claim's jti;
    // JwtAuth::logout does the equivalent when given the access token.
    auth.logout(&pair.access_token).await.unwrap();
    assert!(auth.verify(&pair.access_token).await.is_err());
}

#[tokio::test]
async fn refresh_rotates_and_old_refresh_is_invalid() {
    let (_store, auth) = seeded_store_and_auth().await;
    let pair1 = auth.login("user@example.com", "wos-dev").await.unwrap();
    let pair2 = auth.refresh(&pair1.refresh_token).await.unwrap();
    assert_ne!(pair1.access_token, pair2.access_token);
    assert_ne!(pair1.refresh_token, pair2.refresh_token);

    // Old refresh token should no longer work (the jti was revoked during
    // rotation). Using it again must fail.
    assert!(auth.refresh(&pair1.refresh_token).await.is_err());

    // Fresh access token still verifies.
    assert!(auth.verify(&pair2.access_token).await.is_ok());
}
