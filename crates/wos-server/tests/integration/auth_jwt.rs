//! Auth provider behaviour: login issues a valid token, verify round-trips
//! it, revocation invalidates, refresh rotates jtis.

use std::sync::Arc;

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use chrono::Utc;
use rand::rngs::OsRng;
use wos_server::auth::{AuthError, AuthProvider, JwtAuth};
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
            auth_epoch: 0,
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
async fn verify_returns_database_role_after_upsert_without_auth_epoch_bump() {
    let (store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    assert_eq!(pair.user.role, "Supervisor");

    let mut row = store.get_user("u1").await.unwrap().unwrap();
    row.role = "Operator".into();
    store.upsert_user(&row).await.unwrap();

    let ctx = auth.verify(&pair.access_token).await.unwrap();
    assert_eq!(ctx.user.role, "Operator");
}

#[tokio::test]
async fn verify_includes_avatar_from_user_row() {
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
            avatar: Some("https://cdn.example/a.png".into()),
            auth_epoch: 0,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let handle: wos_server::storage::StorageHandle = store.clone();
    let auth = JwtAuth::new(b"test-secret-not-for-prod", 900, 7 * 24 * 3600, handle);
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    assert_eq!(
        pair.user.avatar.as_deref(),
        Some("https://cdn.example/a.png")
    );
    let ctx = auth.verify(&pair.access_token).await.unwrap();
    assert_eq!(
        ctx.user.avatar.as_deref(),
        Some("https://cdn.example/a.png")
    );
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

    // Logout revokes all session rows for the user so refresh cannot rotate.
    auth.logout(&pair.access_token).await.unwrap();
    assert!(auth.verify(&pair.access_token).await.is_err());
    assert!(auth.refresh(&pair.refresh_token).await.is_err());
}

#[tokio::test]
async fn set_user_password_hash_invalidates_tokens_and_updates_login() {
    let (store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    assert!(auth.verify(&pair.access_token).await.is_ok());

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(b"new-secret", &salt)
        .unwrap()
        .to_string();
    store.set_user_password_hash("u1", &new_hash).await.unwrap();

    assert!(matches!(
        auth.verify(&pair.access_token).await.unwrap_err(),
        AuthError::Revoked
    ));
    assert!(auth.login("user@example.com", "wos-dev").await.is_err());
    assert!(auth.login("user@example.com", "new-secret").await.is_ok());
}

#[tokio::test]
async fn bump_auth_epoch_invalidates_tokens_before_session_revoke() {
    let (store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    assert!(auth.verify(&pair.access_token).await.is_ok());

    store.bump_user_auth_epoch("u1").await.unwrap();
    assert!(matches!(
        auth.verify(&pair.access_token).await.unwrap_err(),
        AuthError::Revoked
    ));
    assert!(matches!(
        auth.refresh(&pair.refresh_token).await.unwrap_err(),
        AuthError::Revoked
    ));
}

#[tokio::test]
async fn logout_rejects_refresh_token_as_body() {
    let (_store, auth) = seeded_store_and_auth().await;
    let pair = auth.login("user@example.com", "wos-dev").await.unwrap();
    let err = auth.logout(&pair.refresh_token).await.unwrap_err();
    assert!(matches!(err, AuthError::InvalidToken));
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
