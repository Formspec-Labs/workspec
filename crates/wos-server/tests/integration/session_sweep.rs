//! WS-052 — daily session-table sweep.
//!
//! `Storage::sweep_expired_sessions` deletes audit-expired session rows so
//! the table cannot grow unboundedly on long-lived deployments. Two
//! deletion windows: any row whose `expires_at < now - 7d`, plus revoked
//! rows whose `expires_at < now - 30d` (longer grace so recent revocations
//! stay queryable during incident response).

use chrono::{Duration, Utc};
use sqlx::Row;
use wos_server::storage::{SessionRow, SqliteStorage, Storage};

async fn fresh() -> SqliteStorage {
    let store = SqliteStorage::connect("sqlite::memory:?cache=shared")
        .await
        .expect("connect in-memory sqlite");
    store.migrate().await.expect("migrate");
    store
}

async fn count_sessions(store: &SqliteStorage) -> i64 {
    sqlx::query("SELECT COUNT(*) FROM sessions")
        .fetch_one(store.pool())
        .await
        .unwrap()
        .try_get::<i64, _>(0)
        .unwrap()
}

#[tokio::test]
async fn sweep_deletes_expired_and_old_revoked_rows() {
    let store = fresh().await;
    let now = Utc::now();

    // Fresh, valid session — should survive.
    store
        .upsert_session(&SessionRow {
            jti: "fresh".into(),
            user_id: "u1".into(),
            expires_at: now + Duration::hours(1),
            revoked: false,
        })
        .await
        .unwrap();

    // Expired 8 days ago — past the 7d unrevoked window, sweep target.
    store
        .upsert_session(&SessionRow {
            jti: "expired-8d".into(),
            user_id: "u1".into(),
            expires_at: now - Duration::days(8),
            revoked: false,
        })
        .await
        .unwrap();

    // Revoked 31 days ago — past the 30d revoked window, sweep target.
    // (Also past the 7d unrevoked window, so qualifies via either branch.)
    store
        .upsert_session(&SessionRow {
            jti: "revoked-31d".into(),
            user_id: "u1".into(),
            expires_at: now - Duration::days(31),
            revoked: true,
        })
        .await
        .unwrap();

    // Revoked 5 days ago, still within the 7d unrevoked window AND the 30d
    // revoked grace window — must survive (audit-relevant for incident review).
    store
        .upsert_session(&SessionRow {
            jti: "revoked-5d".into(),
            user_id: "u1".into(),
            expires_at: now + Duration::days(5),
            revoked: true,
        })
        .await
        .unwrap();

    assert_eq!(count_sessions(&store).await, 4);

    let deleted = store.sweep_expired_sessions(now).await.unwrap();
    assert_eq!(
        deleted, 2,
        "expected expired-8d + revoked-31d to be deleted"
    );

    let remaining = count_sessions(&store).await;
    assert_eq!(remaining, 2);

    // Spot-check that the survivors are the right ones.
    assert!(store.session_is_valid("fresh").await.unwrap());
    assert!(
        sqlx::query("SELECT 1 FROM sessions WHERE jti = ?")
            .bind("revoked-5d")
            .fetch_optional(store.pool())
            .await
            .unwrap()
            .is_some(),
        "revoked-5d should still be present (within 30d grace)"
    );
    assert!(
        sqlx::query("SELECT 1 FROM sessions WHERE jti = ?")
            .bind("expired-8d")
            .fetch_optional(store.pool())
            .await
            .unwrap()
            .is_none(),
        "expired-8d must be gone"
    );
    assert!(
        sqlx::query("SELECT 1 FROM sessions WHERE jti = ?")
            .bind("revoked-31d")
            .fetch_optional(store.pool())
            .await
            .unwrap()
            .is_none(),
        "revoked-31d must be gone"
    );
}

#[tokio::test]
async fn sweep_keeps_active_sessions() {
    let store = fresh().await;
    let now = Utc::now();
    for jti in ["a", "b", "c"] {
        store
            .upsert_session(&SessionRow {
                jti: jti.into(),
                user_id: "u1".into(),
                expires_at: now + Duration::hours(1),
                revoked: false,
            })
            .await
            .unwrap();
    }
    let deleted = store.sweep_expired_sessions(now).await.unwrap();
    assert_eq!(deleted, 0);
    assert_eq!(count_sessions(&store).await, 3);
}
