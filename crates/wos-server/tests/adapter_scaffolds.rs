use std::path::PathBuf;

use wos_server::config::{
    AiChatKind, AuditSinkKind, AuthKind, RuntimeKind, ServerConfig, SignerKind, StorageKind,
};
use wos_server::{auth, storage};

#[tokio::test]
async fn storage_scaffolds_reject_unwired_backends() {
    let cfg = ServerConfig {
        storage: StorageKind::Postgres,
        runtime: RuntimeKind::Local,
        ..minimal_cfg()
    };
    let err = storage::build(&cfg)
        .await
        .err()
        .expect("postgres scaffold must reject");
    let msg = err.to_string();
    assert!(msg.contains("WS-020") || msg.contains("storage-postgres"));

    let cfg = ServerConfig {
        storage: StorageKind::Embedded,
        runtime: RuntimeKind::Local,
        ..minimal_cfg()
    };
    let err = storage::build(&cfg)
        .await
        .err()
        .expect("embedded scaffold must reject");
    assert!(err.to_string().contains("WS-095"));
}

#[tokio::test]
async fn auth_scaffolds_follow_feature_matrix() {
    let mut cfg = minimal_cfg();
    cfg.auth = AuthKind::Jwt;
    let storage = storage::build(&cfg).await.expect("sqlite storage should build");
    let jwt_result = auth::build(&cfg, storage.clone());
    #[cfg(feature = "auth-jwt")]
    assert!(jwt_result.is_ok(), "jwt auth should build when feature is enabled");
    #[cfg(not(feature = "auth-jwt"))]
    {
        let err = match jwt_result {
            Ok(_) => panic!("jwt auth should fail when feature is disabled"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("feature `auth-jwt`"));
    }

    cfg.auth = AuthKind::Mock;
    let mock_result = auth::build(&cfg, storage);
    #[cfg(feature = "auth-mock")]
    assert!(mock_result.is_ok(), "mock auth should build when feature is enabled");
    #[cfg(not(feature = "auth-mock"))]
    {
        let err = match mock_result {
            Ok(_) => panic!("mock auth should fail when feature is disabled"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("feature `auth-mock`"));
    }
}

#[tokio::test]
async fn runtime_restate_startup_matrix_is_explicit() {
    let mut cfg = minimal_cfg();
    cfg.runtime = RuntimeKind::Restate;
    cfg.auth = AuthKind::Mock;
    cfg.jwt_secret = String::new();

    let err = wos_server::run(cfg)
        .await
        .expect_err("runtime=restate should not start in current scaffold state");
    let msg = err.to_string();

    #[cfg(any(feature = "runtime-restate", feature = "runtime-restate-stub"))]
    assert!(
        msg.contains("WS-094 adapter wiring still pending"),
        "expected scaffold-pending signal, got: {msg}"
    );

    #[cfg(not(any(feature = "runtime-restate", feature = "runtime-restate-stub")))]
    assert!(
        msg.contains("without feature `runtime-restate` or `runtime-restate-stub`"),
        "expected feature-off signal, got: {msg}"
    );
}

fn minimal_cfg() -> ServerConfig {
    ServerConfig {
        port: 0,
        fixtures_dir: PathBuf::from("."),
        storage: StorageKind::Sqlite,
        database_url: "sqlite::memory:".into(),
        auth: AuthKind::Jwt,
        jwt_secret: "test-secret".into(),
        jwt_access_ttl_secs: 900,
        jwt_refresh_ttl_secs: 3600,
        cors_origin: "http://localhost:3000".into(),
        cors_strict: false,
        bearer_strict: false,
        seed: false,
        ai_chat: AiChatKind::Disabled,
        gemini_api_key: String::new(),
        cursor_throttle_ms: 50,
        timer_poll_ms: 1000,
        runtime: RuntimeKind::Local,
        audit_sink: AuditSinkKind::None,
        audit_database_url: String::new(),
        session_sweep_enabled: true,
        signer_kind: SignerKind::Noop,
    }
}
