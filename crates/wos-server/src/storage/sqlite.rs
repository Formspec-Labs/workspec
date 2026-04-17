use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use std::str::FromStr;

use super::{
    DelegationRow, InstanceMutator, InstanceQuery, InstanceRow, KernelRow, Page, ProvenanceRow,
    SessionRow, Storage, StorageError, StorageResult, UserRow,
};

pub struct SqliteStorage {
    pool: SqlitePool,
}

impl SqliteStorage {
    pub async fn connect(url: &str) -> StorageResult<Self> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> StorageResult<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn map_kernel(r: &SqliteRow) -> StorageResult<KernelRow> {
    let document_s: String = r.try_get("document")?;
    Ok(KernelRow {
        url: r.try_get("url")?,
        title: r.try_get("title")?,
        version: r.try_get("version")?,
        status: r.try_get("status")?,
        impact_level: r.try_get("impact_level")?,
        document: serde_json::from_str(&document_s)?,
        updated_at: r.try_get::<DateTime<Utc>, _>("updated_at")?,
    })
}

fn map_instance(r: &SqliteRow) -> StorageResult<InstanceRow> {
    Ok(InstanceRow {
        instance_id: r.try_get("instance_id")?,
        definition_url: r.try_get("definition_url")?,
        definition_version: r.try_get("definition_version")?,
        status: r.try_get("status")?,
        impact_level: r.try_get("impact_level")?,
        instance_json: serde_json::from_str(&r.try_get::<String, _>("instance_json")?)?,
        created_at: r.try_get::<DateTime<Utc>, _>("created_at")?,
        updated_at: r.try_get::<DateTime<Utc>, _>("updated_at")?,
    })
}

fn map_provenance(r: &SqliteRow) -> StorageResult<ProvenanceRow> {
    Ok(ProvenanceRow {
        id: r.try_get("id")?,
        instance_id: r.try_get("instance_id")?,
        seq: r.try_get("seq")?,
        timestamp: r.try_get::<DateTime<Utc>, _>("timestamp")?,
        tier: r.try_get("tier")?,
        payload: serde_json::from_str(&r.try_get::<String, _>("payload")?)?,
        hash: r.try_get("hash")?,
        previous_hash: r.try_get("previous_hash")?,
    })
}

fn map_user(r: &SqliteRow) -> StorageResult<UserRow> {
    Ok(UserRow {
        id: r.try_get("id")?,
        email: r.try_get("email")?,
        name: r.try_get("name")?,
        role: r.try_get("role")?,
        password_hash: r.try_get("password_hash")?,
        avatar: r.try_get("avatar")?,
        created_at: r.try_get::<DateTime<Utc>, _>("created_at")?,
    })
}

fn map_delegation(r: &SqliteRow) -> StorageResult<DelegationRow> {
    Ok(DelegationRow {
        id: r.try_get("id")?,
        workflow_url: r.try_get("workflow_url")?,
        delegator: r.try_get("delegator")?,
        delegate: r.try_get("delegate")?,
        scope: r.try_get("scope")?,
        authority: r.try_get("authority")?,
        legal_instrument: r.try_get("legal_instrument")?,
        start_date: r.try_get::<DateTime<Utc>, _>("start_date")?,
        end_date: r.try_get::<Option<DateTime<Utc>>, _>("end_date")?,
        status: r.try_get("status")?,
    })
}

#[async_trait]
impl Storage for SqliteStorage {
    async fn list_kernels(&self) -> StorageResult<Vec<KernelRow>> {
        let rows = sqlx::query("SELECT * FROM kernels ORDER BY url")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(map_kernel).collect()
    }

    async fn get_kernel(&self, url: &str) -> StorageResult<Option<KernelRow>> {
        let row = sqlx::query("SELECT * FROM kernels WHERE url = ?")
            .bind(url)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(map_kernel).transpose()
    }

    async fn upsert_kernel(&self, row: &KernelRow) -> StorageResult<()> {
        let doc = serde_json::to_string(&row.document)?;
        sqlx::query(
            "INSERT INTO kernels (url, title, version, status, impact_level, document, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(url) DO UPDATE SET
               title = excluded.title,
               version = excluded.version,
               status = excluded.status,
               impact_level = excluded.impact_level,
               document = excluded.document,
               updated_at = excluded.updated_at",
        )
        .bind(&row.url)
        .bind(&row.title)
        .bind(&row.version)
        .bind(&row.status)
        .bind(&row.impact_level)
        .bind(&doc)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_instance(&self, row: &InstanceRow) -> StorageResult<()> {
        let instance_json = serde_json::to_string(&row.instance_json)?;
        sqlx::query(
            "INSERT INTO instances (instance_id, definition_url, definition_version, status,
               impact_level, instance_json, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&row.instance_id)
        .bind(&row.definition_url)
        .bind(&row.definition_version)
        .bind(&row.status)
        .bind(&row.impact_level)
        .bind(&instance_json)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_instance(&self, id: &str) -> StorageResult<Option<InstanceRow>> {
        let row = sqlx::query("SELECT * FROM instances WHERE instance_id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(map_instance).transpose()
    }

    async fn list_instances(&self, q: InstanceQuery) -> StorageResult<Page<InstanceRow>> {
        let page = q.page.max(1);
        let page_size = q.page_size.clamp(1, 200);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;

        let mut where_clauses: Vec<String> = Vec::new();
        if let Some(xs) = &q.status {
            where_clauses.push(format!("status IN ({})", vec!["?"; xs.len()].join(",")));
        }
        if let Some(xs) = &q.impact_level {
            where_clauses.push(format!(
                "impact_level IN ({})",
                vec!["?"; xs.len()].join(",")
            ));
        }
        if let Some(xs) = &q.definition_url {
            where_clauses.push(format!(
                "definition_url IN ({})",
                vec!["?"; xs.len()].join(",")
            ));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };

        let count_sql = format!("SELECT COUNT(*) FROM instances {where_sql}");
        let list_sql = format!(
            "SELECT * FROM instances {where_sql} ORDER BY created_at DESC LIMIT ? OFFSET ?"
        );

        let mut count_q = sqlx::query(&count_sql);
        let mut list_q = sqlx::query(&list_sql);
        for bucket in [&q.status, &q.impact_level, &q.definition_url].into_iter() {
            if let Some(xs) = bucket {
                for x in xs {
                    count_q = count_q.bind(x.clone());
                    list_q = list_q.bind(x.clone());
                }
            }
        }
        let count_row = count_q.fetch_one(&self.pool).await?;
        let total: i64 = count_row.try_get(0)?;
        let rows = list_q.bind(limit).bind(offset).fetch_all(&self.pool).await?;
        let items: Vec<InstanceRow> = rows.iter().map(map_instance).collect::<Result<_, _>>()?;

        Ok(Page {
            items,
            total: total as u64,
            page,
            page_size,
        })
    }

    async fn update_instance_atomic(
        &self,
        id: &str,
        mutator: InstanceMutator<'_>,
    ) -> StorageResult<InstanceRow> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query("SELECT * FROM instances WHERE instance_id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        let row = row.ok_or(StorageError::NotFound)?;
        let mut current = map_instance(&row)?;
        let appended = mutator(&mut current)?;
        current.updated_at = Utc::now();

        let instance_json = serde_json::to_string(&current.instance_json)?;
        sqlx::query(
            "UPDATE instances SET
               definition_url = ?, definition_version = ?, status = ?,
               impact_level = ?, instance_json = ?, updated_at = ?
             WHERE instance_id = ?",
        )
        .bind(&current.definition_url)
        .bind(&current.definition_version)
        .bind(&current.status)
        .bind(&current.impact_level)
        .bind(&instance_json)
        .bind(current.updated_at)
        .bind(&current.instance_id)
        .execute(&mut *tx)
        .await?;

        for rec in &appended {
            let payload = serde_json::to_string(&rec.payload)?;
            sqlx::query(
                "INSERT INTO provenance
                    (id, instance_id, seq, timestamp, tier, payload, hash, previous_hash)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&rec.id)
            .bind(&rec.instance_id)
            .bind(rec.seq)
            .bind(rec.timestamp)
            .bind(&rec.tier)
            .bind(&payload)
            .bind(&rec.hash)
            .bind(&rec.previous_hash)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(current)
    }

    async fn list_provenance(&self, instance_id: &str) -> StorageResult<Vec<ProvenanceRow>> {
        let rows = sqlx::query(
            "SELECT * FROM provenance WHERE instance_id = ? ORDER BY seq ASC",
        )
        .bind(instance_id)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_provenance).collect()
    }

    async fn last_provenance(&self, instance_id: &str) -> StorageResult<Option<ProvenanceRow>> {
        let row = sqlx::query(
            "SELECT * FROM provenance WHERE instance_id = ? ORDER BY seq DESC LIMIT 1",
        )
        .bind(instance_id)
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(map_provenance).transpose()
    }

    async fn list_delegations(&self, workflow_url: &str) -> StorageResult<Vec<DelegationRow>> {
        let rows = sqlx::query(
            "SELECT * FROM delegations WHERE workflow_url = ? ORDER BY start_date DESC",
        )
        .bind(workflow_url)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(map_delegation).collect()
    }

    async fn upsert_delegation(&self, row: &DelegationRow) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO delegations
               (id, workflow_url, delegator, delegate, scope, authority,
                legal_instrument, start_date, end_date, status)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               workflow_url = excluded.workflow_url,
               delegator = excluded.delegator,
               delegate = excluded.delegate,
               scope = excluded.scope,
               authority = excluded.authority,
               legal_instrument = excluded.legal_instrument,
               start_date = excluded.start_date,
               end_date = excluded.end_date,
               status = excluded.status",
        )
        .bind(&row.id)
        .bind(&row.workflow_url)
        .bind(&row.delegator)
        .bind(&row.delegate)
        .bind(&row.scope)
        .bind(&row.authority)
        .bind(&row.legal_instrument)
        .bind(row.start_date)
        .bind(row.end_date)
        .bind(&row.status)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_delegation(&self, workflow_url: &str, id: &str) -> StorageResult<()> {
        sqlx::query(
            "UPDATE delegations SET status = 'revoked' WHERE workflow_url = ? AND id = ?",
        )
        .bind(workflow_url)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_user_by_email(&self, email: &str) -> StorageResult<Option<UserRow>> {
        let row = sqlx::query("SELECT * FROM users WHERE lower(email) = lower(?)")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(map_user).transpose()
    }

    async fn get_user(&self, id: &str) -> StorageResult<Option<UserRow>> {
        let row = sqlx::query("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(map_user).transpose()
    }

    async fn upsert_user(&self, row: &UserRow) -> StorageResult<()> {
        sqlx::query(
            "INSERT INTO users (id, email, name, role, password_hash, avatar, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               email = excluded.email,
               name = excluded.name,
               role = excluded.role,
               password_hash = excluded.password_hash,
               avatar = excluded.avatar",
        )
        .bind(&row.id)
        .bind(&row.email)
        .bind(&row.name)
        .bind(&row.role)
        .bind(&row.password_hash)
        .bind(&row.avatar)
        .bind(row.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn upsert_session(&self, row: &SessionRow) -> StorageResult<()> {
        let revoked: i64 = if row.revoked { 1 } else { 0 };
        sqlx::query(
            "INSERT INTO sessions (jti, user_id, expires_at, revoked)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(jti) DO UPDATE SET
               user_id = excluded.user_id,
               expires_at = excluded.expires_at,
               revoked = excluded.revoked",
        )
        .bind(&row.jti)
        .bind(&row.user_id)
        .bind(row.expires_at)
        .bind(revoked)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_session(&self, jti: &str) -> StorageResult<()> {
        sqlx::query("UPDATE sessions SET revoked = 1 WHERE jti = ?")
            .bind(jti)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn session_is_valid(&self, jti: &str) -> StorageResult<bool> {
        let row = sqlx::query("SELECT revoked, expires_at FROM sessions WHERE jti = ?")
            .bind(jti)
            .fetch_optional(&self.pool)
            .await?;
        Ok(match row {
            Some(r) => {
                let revoked: i64 = r.try_get("revoked")?;
                let expires: DateTime<Utc> = r.try_get("expires_at")?;
                revoked == 0 && expires > Utc::now()
            }
            None => false,
        })
    }
}
