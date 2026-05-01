//! Seed data: fixture kernels are ingested by [`BundleService::hydrate`] on
//! every boot; this module additionally inserts demo users and (optionally)
//! demo instances when the store is empty.

use argon2::{Argon2, PasswordHasher, password_hash::SaltString};
use chrono::Utc;
use rand::rngs::OsRng;

use crate::AppState;
use crate::storage::{InstanceRow, UserRow};

/// Run once at boot when `--seed` is set. Idempotent: each demo record is
/// only inserted when a matching id is absent from the store.
pub async fn run(state: &AppState) -> anyhow::Result<()> {
    seed_users(state).await?;
    seed_instances(state).await?;
    Ok(())
}

async fn seed_users(state: &AppState) -> anyhow::Result<()> {
    let demo = [
        (
            "user-jane-doe",
            "jane.doe@example.gov",
            "Jane Doe",
            "Supervisor",
        ),
        (
            "user-sarah-j",
            "sarah.jenkins@example.gov",
            "Sarah Jenkins",
            "Caseworker",
        ),
        (
            "user-maria-a",
            "maria.applicant@example.com",
            "Maria Applicant",
            "Applicant",
        ),
    ];
    for (id, email, name, role) in demo {
        if state.storage.get_user(id).await?.is_some() {
            continue;
        }
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"wos-dev", &salt)
            .map_err(|e| anyhow::anyhow!("password hash failed: {e}"))?
            .to_string();
        state
            .storage
            .upsert_user(&UserRow {
                id: id.into(),
                email: email.into(),
                name: name.into(),
                role: role.into(),
                password_hash: hash,
                avatar: None,
                auth_epoch: 0,
                created_at: Utc::now(),
            })
            .await?;
    }
    tracing::info!("seeded 3 demo users (shared dev password is documented in wos-server README)");
    Ok(())
}

async fn seed_instances(state: &AppState) -> anyhow::Result<()> {
    // Only seed if no instances exist. Keeps the seed flag idempotent without
    // stepping on real tenant data.
    let existing = state
        .storage
        .list_instances(crate::storage::InstanceQuery {
            page: 1,
            page_size: 1,
            ..Default::default()
        })
        .await?;
    if existing.total > 0 {
        return Ok(());
    }

    let Some(primary) = state.services.bundle.primary_kernel().await else {
        tracing::warn!("no kernels loaded; skipping instance seed");
        return Ok(());
    };

    for (suffix, configuration, case_state) in [
        (
            "a1b2c3d4",
            vec!["intake"],
            serde_json::json!({ "applicantId": "app-maria-a" }),
        ),
        (
            "e5f6g7h8",
            vec!["intake"],
            serde_json::json!({ "applicantId": "app-demo-002" }),
        ),
        (
            "i9j0k1l2",
            vec!["adverseNotice"],
            serde_json::json!({ "applicantId": "app-demo-003", "decision": "denied" }),
        ),
    ] {
        let instance_id = format!("urn:wos:instance:{}:{suffix}", url_to_slug(&primary.url));
        let now = Utc::now();
        let snapshot = serde_json::json!({
            "instanceId": instance_id,
            "definitionUrl": primary.url,
            "definitionVersion": primary.version,
            "configuration": configuration,
            "caseState": case_state,
            "provenancePosition": 0,
            "nextTaskSequence": 0,
            "timers": [],
            "activeTasks": [],
            "historyStore": {},
            "compensationLogs": {},
            "status": "active",
            "pendingEvents": [],
            "governanceState": null,
            "volumeCounters": null,
            "createdAt": now.to_rfc3339(),
            "updatedAt": now.to_rfc3339(),
            "firedMilestones": [],
            "pendingCallbacks": {},
            "extensions": {},
        });

        state
            .storage
            .create_instance(&InstanceRow {
                instance_id: instance_id.clone(),
                definition_url: primary.url.clone(),
                definition_version: primary.version.clone(),
                status: "active".into(),
                impact_level: primary.impact_level.clone(),
                instance_json: snapshot,
                runtime_aux_json: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            })
            .await?;
    }
    tracing::info!("seeded 3 demo instances");
    Ok(())
}

fn url_to_slug(url: &str) -> String {
    url.rsplit(':').nth(1).map(String::from).unwrap_or_else(|| {
        url.rsplit('/')
            .next()
            .unwrap_or(url)
            .trim_end_matches(".json")
            .to_string()
    })
}
