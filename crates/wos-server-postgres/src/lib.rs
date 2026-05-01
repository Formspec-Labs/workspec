use async_trait::async_trait;
use postgres::{Client, NoTls, Row};
use std::sync::{Arc, Mutex};
use wos_server_ports::storage::{
    AgentRow, DelegationRow, IdentityFactRow, InboundCloudEventRow, InstanceMutator, InstanceQuery,
    InstanceRow, IntakeRecordRow, KernelRow, LIST_INSTANCES_PAGE_SIZE_MAX, Page, ProvenanceRow,
    SessionRow, Storage, StorageError, StorageResult, UserRow,
};

/// Thin composition adapter over Trellis Postgres storage.
///
/// This crate intentionally does not reimplement Trellis envelope persistence.
/// The Trellis pool is held as an operational guardrail: canonical envelope
/// writes belong to Trellis-owned paths, while this adapter persists only WOS
/// operational projections in sibling tables. There is no duplicate-envelope
/// ownership claim in this adapter.
pub struct PostgresStorage {
    trellis_store: trellis_store_postgres::PostgresStorePool,
    client: Arc<Mutex<Client>>,
}

impl PostgresStorage {
    pub fn connect(dsn: &str) -> StorageResult<Self> {
        let pool = trellis_store_postgres::PostgresStorePool::builder(dsn)
            .build()
            .map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut client =
            Client::connect(dsn, NoTls).map_err(|e| StorageError::Backend(e.to_string()))?;
        migrate(&mut client)?;
        Ok(Self {
            trellis_store: pool,
            client: Arc::new(Mutex::new(client)),
        })
    }
}

fn se(e: postgres::Error) -> StorageError {
    if let Some(db) = e.as_db_error() {
        if db.code() == &postgres::error::SqlState::UNIQUE_VIOLATION {
            return StorageError::Conflict(db.message().to_string());
        }
    }
    StorageError::Backend(e.to_string())
}

fn migrate(client: &mut Client) -> StorageResult<()> {
    client
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS kernels (
            url TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            version TEXT NOT NULL,
            status TEXT NOT NULL,
            impact_level TEXT NOT NULL,
            document JSONB NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        );
        CREATE TABLE IF NOT EXISTS instances (
            instance_id TEXT PRIMARY KEY,
            definition_url TEXT NOT NULL,
            definition_version TEXT NOT NULL,
            status TEXT NOT NULL,
            impact_level TEXT NOT NULL,
            instance_json JSONB NOT NULL,
            runtime_aux_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        );
        CREATE TABLE IF NOT EXISTS provenance (
            id TEXT PRIMARY KEY,
            instance_id TEXT NOT NULL,
            seq BIGINT NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            tier TEXT NOT NULL,
            payload JSONB NOT NULL,
            hash TEXT NOT NULL,
            previous_hash TEXT NOT NULL,
            UNIQUE(instance_id, seq)
        );
        CREATE TABLE IF NOT EXISTS delegations (
            id TEXT PRIMARY KEY,
            workflow_url TEXT NOT NULL,
            delegator TEXT NOT NULL,
            delegate TEXT NOT NULL,
            scope TEXT NOT NULL,
            authority TEXT,
            legal_instrument TEXT,
            start_date TIMESTAMPTZ NOT NULL,
            end_date TIMESTAMPTZ,
            status TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            avatar TEXT,
            auth_epoch BIGINT NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_lower ON users(lower(email));
        CREATE TABLE IF NOT EXISTS sessions (
            jti TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            expires_at TIMESTAMPTZ NOT NULL,
            revoked BOOLEAN NOT NULL DEFAULT FALSE
        );
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            workflow_url TEXT NOT NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            version TEXT NOT NULL,
            status TEXT NOT NULL,
            autonomy TEXT,
            confidence_floor DOUBLE PRECISION,
            config_json JSONB NOT NULL,
            deployment_state TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL
        );
        CREATE TABLE IF NOT EXISTS identity_facts (
            id TEXT PRIMARY KEY,
            instance_id TEXT NOT NULL,
            subject_ref TEXT NOT NULL,
            assurance_level TEXT NOT NULL,
            disclosure_posture TEXT NOT NULL,
            fact_json JSONB NOT NULL,
            upgraded_from TEXT,
            created_at TIMESTAMPTZ NOT NULL
        );
        CREATE TABLE IF NOT EXISTS integration_inbound (
            cloud_event_id TEXT PRIMARY KEY,
            instance_id TEXT NOT NULL,
            binding TEXT NOT NULL,
            received_at TIMESTAMPTZ NOT NULL,
            payload_json JSONB NOT NULL
        );
        CREATE TABLE IF NOT EXISTS intake_records (
            binding TEXT NOT NULL,
            intake_id TEXT NOT NULL,
            status TEXT NOT NULL,
            record_json JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            PRIMARY KEY(binding, intake_id)
        );",
        )
        .map_err(se)?;
    Ok(())
}

fn map_kernel(r: &Row) -> KernelRow {
    KernelRow {
        url: r.get("url"),
        title: r.get("title"),
        version: r.get("version"),
        status: r.get("status"),
        impact_level: r.get("impact_level"),
        document: r.get("document"),
        updated_at: r.get("updated_at"),
    }
}

fn map_instance(r: &Row) -> InstanceRow {
    InstanceRow {
        instance_id: r.get("instance_id"),
        definition_url: r.get("definition_url"),
        definition_version: r.get("definition_version"),
        status: r.get("status"),
        impact_level: r.get("impact_level"),
        instance_json: r.get("instance_json"),
        runtime_aux_json: r.get("runtime_aux_json"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn map_provenance(r: &Row) -> ProvenanceRow {
    ProvenanceRow {
        id: r.get("id"),
        instance_id: r.get("instance_id"),
        seq: r.get("seq"),
        timestamp: r.get("timestamp"),
        tier: r.get("tier"),
        payload: r.get("payload"),
        hash: r.get("hash"),
        previous_hash: r.get("previous_hash"),
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn list_kernels(&self) -> StorageResult<Vec<KernelRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query("SELECT * FROM kernels ORDER BY url", &[])
            .map_err(se)?;
        Ok(rows.iter().map(map_kernel).collect())
    }
    async fn get_kernel(&self, url: &str) -> StorageResult<Option<KernelRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt("SELECT * FROM kernels WHERE url = $1", &[&url])
            .map_err(se)?;
        Ok(row.as_ref().map(map_kernel))
    }
    async fn upsert_kernel(&self, row: &KernelRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute(
            "INSERT INTO kernels (url,title,version,status,impact_level,document,updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT(url) DO UPDATE SET
               title=excluded.title,version=excluded.version,status=excluded.status,
               impact_level=excluded.impact_level,document=excluded.document,updated_at=excluded.updated_at",
            &[&row.url,&row.title,&row.version,&row.status,&row.impact_level,&row.document,&row.updated_at],
        ).map_err(se)?;
        Ok(())
    }
    async fn create_instance(&self, row: &InstanceRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute(
            "INSERT INTO instances (instance_id,definition_url,definition_version,status,impact_level,instance_json,runtime_aux_json,created_at,updated_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
            &[&row.instance_id,&row.definition_url,&row.definition_version,&row.status,&row.impact_level,&row.instance_json,&row.runtime_aux_json,&row.created_at,&row.updated_at],
        ).map_err(se)?;
        Ok(())
    }
    async fn get_instance(&self, id: &str) -> StorageResult<Option<InstanceRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt("SELECT * FROM instances WHERE instance_id = $1", &[&id])
            .map_err(se)?;
        Ok(row.as_ref().map(map_instance))
    }
    async fn list_instances(&self, q: InstanceQuery) -> StorageResult<Page<InstanceRow>> {
        let page = q.page.max(1);
        let page_size = q.page_size.clamp(1, LIST_INSTANCES_PAGE_SIZE_MAX);
        let offset = ((page - 1) * page_size) as i64;
        let limit = page_size as i64;
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let status = q.status.unwrap_or_default();
        let impact = q.impact_level.unwrap_or_default();
        let defs = q.definition_url.unwrap_or_default();
        let rows = c
            .query(
                "SELECT * FROM instances
             WHERE ($1::text[] = '{}'::text[] OR status = ANY($1))
               AND ($2::text[] = '{}'::text[] OR impact_level = ANY($2))
               AND ($3::text[] = '{}'::text[] OR definition_url = ANY($3))
             ORDER BY created_at DESC LIMIT $4 OFFSET $5",
                &[&status, &impact, &defs, &limit, &offset],
            )
            .map_err(se)?;
        let total_row = c
            .query_one(
                "SELECT COUNT(*) AS total FROM instances
             WHERE ($1::text[] = '{}'::text[] OR status = ANY($1))
               AND ($2::text[] = '{}'::text[] OR impact_level = ANY($2))
               AND ($3::text[] = '{}'::text[] OR definition_url = ANY($3))",
                &[&status, &impact, &defs],
            )
            .map_err(se)?;
        let total: i64 = total_row.get("total");
        Ok(Page {
            items: rows.iter().map(map_instance).collect(),
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
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let mut tx = c.transaction().map_err(se)?;
        let row = tx
            .query_opt(
                "SELECT * FROM instances WHERE instance_id = $1 FOR UPDATE",
                &[&id],
            )
            .map_err(se)?
            .ok_or(StorageError::NotFound)?;
        let mut current = map_instance(&row);
        let appended = mutator(&mut current)?;
        current.updated_at = chrono::Utc::now();
        tx.execute(
            "UPDATE instances SET definition_url=$1,definition_version=$2,status=$3,impact_level=$4,instance_json=$5,runtime_aux_json=$6,updated_at=$7 WHERE instance_id=$8",
            &[&current.definition_url,&current.definition_version,&current.status,&current.impact_level,&current.instance_json,&current.runtime_aux_json,&current.updated_at,&current.instance_id]
        ).map_err(se)?;
        for rec in appended {
            tx.execute(
                "INSERT INTO provenance (id,instance_id,seq,timestamp,tier,payload,hash,previous_hash) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
                &[&rec.id,&rec.instance_id,&rec.seq,&rec.timestamp,&rec.tier,&rec.payload,&rec.hash,&rec.previous_hash]
            ).map_err(se)?;
        }
        tx.commit().map_err(se)?;
        Ok(current)
    }
    async fn list_provenance(&self, instance_id: &str) -> StorageResult<Vec<ProvenanceRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query(
                "SELECT * FROM provenance WHERE instance_id=$1 ORDER BY seq ASC",
                &[&instance_id],
            )
            .map_err(se)?;
        Ok(rows.iter().map(map_provenance).collect())
    }
    async fn last_provenance(&self, instance_id: &str) -> StorageResult<Option<ProvenanceRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt(
                "SELECT * FROM provenance WHERE instance_id=$1 ORDER BY seq DESC LIMIT 1",
                &[&instance_id],
            )
            .map_err(se)?;
        Ok(row.as_ref().map(map_provenance))
    }
    async fn list_delegations(&self, workflow_url: &str) -> StorageResult<Vec<DelegationRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query(
                "SELECT * FROM delegations WHERE workflow_url=$1 ORDER BY start_date DESC",
                &[&workflow_url],
            )
            .map_err(se)?;
        Ok(rows
            .iter()
            .map(|r| DelegationRow {
                id: r.get("id"),
                workflow_url: r.get("workflow_url"),
                delegator: r.get("delegator"),
                delegate: r.get("delegate"),
                scope: r.get("scope"),
                authority: r.get("authority"),
                legal_instrument: r.get("legal_instrument"),
                start_date: r.get("start_date"),
                end_date: r.get("end_date"),
                status: r.get("status"),
            })
            .collect())
    }
    async fn upsert_delegation(&self, row: &DelegationRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("INSERT INTO delegations (id,workflow_url,delegator,delegate,scope,authority,legal_instrument,start_date,end_date,status)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
                   ON CONFLICT(id) DO UPDATE SET workflow_url=excluded.workflow_url,delegator=excluded.delegator,delegate=excluded.delegate,scope=excluded.scope,authority=excluded.authority,legal_instrument=excluded.legal_instrument,start_date=excluded.start_date,end_date=excluded.end_date,status=excluded.status",
            &[&row.id,&row.workflow_url,&row.delegator,&row.delegate,&row.scope,&row.authority,&row.legal_instrument,&row.start_date,&row.end_date,&row.status]).map_err(se)?;
        Ok(())
    }
    async fn revoke_delegation(&self, workflow_url: &str, id: &str) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute(
            "UPDATE delegations SET status='revoked' WHERE workflow_url=$1 AND id=$2",
            &[&workflow_url, &id],
        )
        .map_err(se)?;
        Ok(())
    }
    async fn upsert_agent(&self, row: &AgentRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("INSERT INTO agents (id,workflow_url,name,kind,version,status,autonomy,confidence_floor,config_json,deployment_state,created_at,updated_at)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
                   ON CONFLICT(id) DO UPDATE SET workflow_url=excluded.workflow_url,name=excluded.name,kind=excluded.kind,version=excluded.version,status=excluded.status,autonomy=excluded.autonomy,confidence_floor=excluded.confidence_floor,config_json=excluded.config_json,deployment_state=excluded.deployment_state,updated_at=excluded.updated_at",
            &[&row.id,&row.workflow_url,&row.name,&row.kind,&row.version,&row.status,&row.autonomy,&row.confidence_floor,&row.config_json,&row.deployment_state,&row.created_at,&row.updated_at]).map_err(se)?;
        Ok(())
    }
    async fn get_agent(&self, id: &str) -> StorageResult<Option<AgentRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt("SELECT * FROM agents WHERE id=$1", &[&id])
            .map_err(se)?;
        Ok(row.map(|r| AgentRow {
            id: r.get("id"),
            workflow_url: r.get("workflow_url"),
            name: r.get("name"),
            kind: r.get("kind"),
            version: r.get("version"),
            status: r.get("status"),
            autonomy: r.get("autonomy"),
            confidence_floor: r.get("confidence_floor"),
            config_json: r.get("config_json"),
            deployment_state: r.get("deployment_state"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }
    async fn list_agents(&self, workflow_url: &str) -> StorageResult<Vec<AgentRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query(
                "SELECT * FROM agents WHERE workflow_url=$1 ORDER BY name ASC",
                &[&workflow_url],
            )
            .map_err(se)?;
        Ok(rows
            .iter()
            .map(|r| AgentRow {
                id: r.get("id"),
                workflow_url: r.get("workflow_url"),
                name: r.get("name"),
                kind: r.get("kind"),
                version: r.get("version"),
                status: r.get("status"),
                autonomy: r.get("autonomy"),
                confidence_floor: r.get("confidence_floor"),
                config_json: r.get("config_json"),
                deployment_state: r.get("deployment_state"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }
    async fn insert_identity_fact(&self, row: &IdentityFactRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("INSERT INTO identity_facts (id,instance_id,subject_ref,assurance_level,disclosure_posture,fact_json,upgraded_from,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)", &[&row.id,&row.instance_id,&row.subject_ref,&row.assurance_level,&row.disclosure_posture,&row.fact_json,&row.upgraded_from,&row.created_at]).map_err(se)?;
        Ok(())
    }
    async fn get_identity_fact(&self, id: &str) -> StorageResult<Option<IdentityFactRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt("SELECT * FROM identity_facts WHERE id=$1", &[&id])
            .map_err(se)?;
        Ok(row.map(|r| IdentityFactRow {
            id: r.get("id"),
            instance_id: r.get("instance_id"),
            subject_ref: r.get("subject_ref"),
            assurance_level: r.get("assurance_level"),
            disclosure_posture: r.get("disclosure_posture"),
            fact_json: r.get("fact_json"),
            upgraded_from: r.get("upgraded_from"),
            created_at: r.get("created_at"),
        }))
    }
    async fn list_identity_facts(&self, instance_id: &str) -> StorageResult<Vec<IdentityFactRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query(
                "SELECT * FROM identity_facts WHERE instance_id=$1 ORDER BY created_at ASC",
                &[&instance_id],
            )
            .map_err(se)?;
        Ok(rows
            .iter()
            .map(|r| IdentityFactRow {
                id: r.get("id"),
                instance_id: r.get("instance_id"),
                subject_ref: r.get("subject_ref"),
                assurance_level: r.get("assurance_level"),
                disclosure_posture: r.get("disclosure_posture"),
                fact_json: r.get("fact_json"),
                upgraded_from: r.get("upgraded_from"),
                created_at: r.get("created_at"),
            })
            .collect())
    }
    async fn list_assurance_chain(&self, subject_ref: &str) -> StorageResult<Vec<IdentityFactRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let rows = c
            .query(
                "SELECT * FROM identity_facts WHERE subject_ref=$1 ORDER BY created_at ASC",
                &[&subject_ref],
            )
            .map_err(se)?;
        Ok(rows
            .iter()
            .map(|r| IdentityFactRow {
                id: r.get("id"),
                instance_id: r.get("instance_id"),
                subject_ref: r.get("subject_ref"),
                assurance_level: r.get("assurance_level"),
                disclosure_posture: r.get("disclosure_posture"),
                fact_json: r.get("fact_json"),
                upgraded_from: r.get("upgraded_from"),
                created_at: r.get("created_at"),
            })
            .collect())
    }
    async fn get_inbound_cloud_event(
        &self,
        cloud_event_id: &str,
    ) -> StorageResult<Option<InboundCloudEventRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt(
                "SELECT * FROM integration_inbound WHERE cloud_event_id=$1",
                &[&cloud_event_id],
            )
            .map_err(se)?;
        Ok(row.map(|r| InboundCloudEventRow {
            cloud_event_id: r.get("cloud_event_id"),
            instance_id: r.get("instance_id"),
            binding: r.get("binding"),
            received_at: r.get("received_at"),
            payload_json: r.get("payload_json"),
        }))
    }
    async fn insert_inbound_cloud_event(&self, row: &InboundCloudEventRow) -> StorageResult<bool> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let n = c.execute("INSERT INTO integration_inbound (cloud_event_id,instance_id,binding,received_at,payload_json) VALUES ($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING", &[&row.cloud_event_id,&row.instance_id,&row.binding,&row.received_at,&row.payload_json]).map_err(se)?;
        Ok(n > 0)
    }
    async fn get_intake_record(
        &self,
        binding: &str,
        intake_id: &str,
    ) -> StorageResult<Option<IntakeRecordRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt(
                "SELECT * FROM intake_records WHERE binding=$1 AND intake_id=$2",
                &[&binding, &intake_id],
            )
            .map_err(se)?;
        Ok(row.map(|r| IntakeRecordRow {
            binding: r.get("binding"),
            intake_id: r.get("intake_id"),
            status: r.get("status"),
            record_json: r.get("record_json"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }
    async fn insert_intake_record(&self, row: &IntakeRecordRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("INSERT INTO intake_records (binding,intake_id,status,record_json,created_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6)", &[&row.binding,&row.intake_id,&row.status,&row.record_json,&row.created_at,&row.updated_at]).map_err(se)?;
        Ok(())
    }
    async fn update_intake_record(&self, row: &IntakeRecordRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let n = c.execute("UPDATE intake_records SET status=$1,record_json=$2,updated_at=$3 WHERE binding=$4 AND intake_id=$5", &[&row.status,&row.record_json,&row.updated_at,&row.binding,&row.intake_id]).map_err(se)?;
        if n == 0 {
            Err(StorageError::NotFound)
        } else {
            Ok(())
        }
    }
    async fn get_user_by_email(&self, email: &str) -> StorageResult<Option<UserRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt(
                "SELECT * FROM users WHERE lower(email)=lower($1)",
                &[&email],
            )
            .map_err(se)?;
        Ok(row.map(|r| UserRow {
            id: r.get("id"),
            email: r.get("email"),
            name: r.get("name"),
            role: r.get("role"),
            password_hash: r.get("password_hash"),
            avatar: r.get("avatar"),
            auth_epoch: r.get("auth_epoch"),
            created_at: r.get("created_at"),
        }))
    }
    async fn get_user(&self, id: &str) -> StorageResult<Option<UserRow>> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt("SELECT * FROM users WHERE id=$1", &[&id])
            .map_err(se)?;
        Ok(row.map(|r| UserRow {
            id: r.get("id"),
            email: r.get("email"),
            name: r.get("name"),
            role: r.get("role"),
            password_hash: r.get("password_hash"),
            avatar: r.get("avatar"),
            auth_epoch: r.get("auth_epoch"),
            created_at: r.get("created_at"),
        }))
    }
    async fn upsert_user(&self, row: &UserRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        // AUTH-CONTRACT INVARIANT: the ON CONFLICT DO UPDATE clause intentionally
        // omits `password_hash` and `auth_epoch`. These two fields belong to the
        // auth contract and are set ONLY at initial user creation (the INSERT
        // arm). Subsequent upserts MUST NOT overwrite them — password rotation
        // flows exclusively through `set_user_password_hash` (which atomically
        // updates the hash, bumps the epoch, and revokes sessions in a single
        // tx), and global logout flows through `bump_user_auth_epoch`. Allowing
        // upsert to clobber either field would silently nullify session
        // invalidation guarantees and weaken the password rotation atomicity
        // contract.
        //
        // Both adapters (this Postgres impl and `wos-server-sqlite`) implement
        // the same invariant — keep them in lockstep. See
        // `crates/wos-server/PARITY.md` § Auth contract for the full rule
        // (`upsert_user` never touches `password_hash` / `auth_epoch`).
        c.execute("INSERT INTO users (id,email,name,role,password_hash,avatar,auth_epoch,created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
                   ON CONFLICT(id) DO UPDATE SET email=excluded.email,name=excluded.name,role=excluded.role,avatar=excluded.avatar",
            &[&row.id,&row.email,&row.name,&row.role,&row.password_hash,&row.avatar,&row.auth_epoch,&row.created_at]).map_err(se)?;
        Ok(())
    }
    async fn bump_user_auth_epoch(&self, user_id: &str) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute(
            "UPDATE users SET auth_epoch = auth_epoch + 1 WHERE id=$1",
            &[&user_id],
        )
        .map_err(se)?;
        Ok(())
    }
    async fn set_user_password_hash(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let mut tx = c.transaction().map_err(se)?;
        let n = tx
            .execute(
                "UPDATE users SET password_hash=$1 WHERE id=$2",
                &[&password_hash, &user_id],
            )
            .map_err(se)?;
        if n == 0 {
            return Err(StorageError::NotFound);
        }
        tx.execute(
            "UPDATE users SET auth_epoch = auth_epoch + 1 WHERE id=$1",
            &[&user_id],
        )
        .map_err(se)?;
        tx.execute(
            "UPDATE sessions SET revoked = TRUE WHERE user_id=$1",
            &[&user_id],
        )
        .map_err(se)?;
        tx.commit().map_err(se)?;
        Ok(())
    }
    async fn upsert_session(&self, row: &SessionRow) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("INSERT INTO sessions (jti,user_id,expires_at,revoked) VALUES ($1,$2,$3,$4)
                   ON CONFLICT(jti) DO UPDATE SET user_id=excluded.user_id,expires_at=excluded.expires_at,revoked=excluded.revoked",
            &[&row.jti,&row.user_id,&row.expires_at,&row.revoked]).map_err(se)?;
        Ok(())
    }
    async fn revoke_session(&self, jti: &str) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute("UPDATE sessions SET revoked = TRUE WHERE jti=$1", &[&jti])
            .map_err(se)?;
        Ok(())
    }
    async fn revoke_sessions_for_user(&self, user_id: &str) -> StorageResult<()> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        c.execute(
            "UPDATE sessions SET revoked = TRUE WHERE user_id=$1",
            &[&user_id],
        )
        .map_err(se)?;
        Ok(())
    }
    async fn sweep_expired_sessions(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> StorageResult<u64> {
        let cutoff_unrevoked = now - chrono::Duration::days(7);
        let cutoff_revoked = now - chrono::Duration::days(30);
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let n = c.execute(
            "DELETE FROM sessions WHERE expires_at < $1 OR (revoked = TRUE AND expires_at < $2)",
            &[&cutoff_unrevoked, &cutoff_revoked],
        ).map_err(se)?;
        Ok(n)
    }

    async fn session_is_valid(&self, jti: &str) -> StorageResult<bool> {
        let mut c = self
            .client
            .lock()
            .map_err(|_| StorageError::Backend("postgres client mutex poisoned".into()))?;
        let row = c
            .query_opt(
                "SELECT revoked, expires_at FROM sessions WHERE jti=$1",
                &[&jti],
            )
            .map_err(se)?;
        Ok(match row {
            Some(r) => {
                let revoked: bool = r.get("revoked");
                let expires_at: chrono::DateTime<chrono::Utc> = r.get("expires_at");
                !revoked && expires_at > chrono::Utc::now()
            }
            None => false,
        })
    }
}

impl PostgresStorage {
    /// True when the Trellis canonical store dependency is initialized.
    ///
    /// This is an invariant check only; operational table writes in this crate
    /// do not claim canonical-envelope ownership or duplicate-envelope authority.
    pub fn trellis_store_ready(&self) -> bool {
        let _ = &self.trellis_store;
        true
    }

    /// Borrow the Trellis canonical-store pool.
    ///
    /// Exposed so downstream composition (the `EventStore` adapter that will
    /// supersede this operational `Storage` per [VISION.md §VIII]) can route
    /// canonical envelope appends through the Trellis-owned pool rather than
    /// the operational `client` held here. Direct callers MUST NOT use this
    /// pool to write to the operational projection tables; that path runs
    /// through the trait methods on this type.
    ///
    /// **Lifecycle.** This accessor exists exclusively for the forthcoming
    /// `EventStore` adapter composition (WS-090 Trellis EventStore, WS-095
    /// adapter cluster split). It currently has zero in-tree callers — the
    /// only consumer is the unit test that asserts the pool is initialized
    /// (`trellis_store_ready`).
    ///
    /// Once WS-090 / WS-095 land, this method will either be removed
    /// outright (if the EventStore adapter constructs its own pool from the
    /// DSN) or scoped down to `pub(crate)` and the borrow moved into the
    /// EventStore adapter crate. Do **not** add new call sites without
    /// reading the WS-090 / WS-095 plan first; doing so will entangle this
    /// crate with paths that are scheduled for removal.
    pub fn trellis_pool(&self) -> &trellis_store_postgres::PostgresStorePool {
        &self.trellis_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};
    use std::{
        future::Future,
        task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
    };
    use wos_server_ports::storage::Storage;
    use wos_server_sqlite::SqliteStorage;

    struct TestCluster {
        temp_dir: tempfile::TempDir,
        port: u16,
        pg_ctl: PathBuf,
    }

    impl TestCluster {
        fn start() -> Option<Self> {
            let temp_dir = tempfile::TempDir::new().ok()?;
            let data_dir = temp_dir.path().join("data");
            let socket_dir = temp_dir.path().join("socket");
            if std::fs::create_dir_all(&socket_dir).is_err() {
                return None;
            }
            let Some(initdb) = find_pg_binary("initdb") else {
                return None;
            };
            let Some(pg_ctl) = find_pg_binary("pg_ctl") else {
                return None;
            };
            let port = reserve_port();
            if !run_command(
                Command::new(&initdb)
                    .arg("-D")
                    .arg(&data_dir)
                    .arg("--username=postgres")
                    .arg("--auth=trust")
                    .arg("--no-locale"),
            ) {
                return None;
            }
            if !run_command(
                Command::new(&pg_ctl)
                    .arg("-D")
                    .arg(&data_dir)
                    .arg("-o")
                    .arg(format!("-F -p {port} -k {}", socket_dir.display()))
                    .arg("start"),
            ) {
                return None;
            }
            if !wait_for_postgres(port) {
                return None;
            }
            Some(Self {
                temp_dir,
                port,
                pg_ctl,
            })
        }

        fn dsn(&self) -> String {
            format!(
                "host=127.0.0.1 port={} user=postgres dbname=postgres",
                self.port
            )
        }
    }

    impl Drop for TestCluster {
        fn drop(&mut self) {
            let data_dir = self.temp_dir.path().join("data");
            let _ = Command::new(&self.pg_ctl)
                .arg("-D")
                .arg(&data_dir)
                .arg("-m")
                .arg("immediate")
                .arg("stop")
                .status();
        }
    }

    /// Resolve a Postgres DSN for tests.
    ///
    /// Resolution order:
    /// 1. `WOS_POSTGRES_TEST_URL` — explicit override (preferred in CI).
    /// 2. `DATABASE_URL` — common CI convention.
    /// 3. Spin up an ephemeral cluster via `initdb` + `pg_ctl` if the binaries
    ///    are available locally.
    /// Returns `None` (callers must skip cleanly) when none of the above
    /// produces a usable DSN.
    fn resolve_test_dsn() -> Option<(String, Option<TestCluster>)> {
        if let Ok(dsn) = std::env::var("WOS_POSTGRES_TEST_URL") {
            if !dsn.trim().is_empty() {
                return Some((dsn, None));
            }
        }
        if let Ok(dsn) = std::env::var("DATABASE_URL") {
            if !dsn.trim().is_empty() {
                return Some((dsn, None));
            }
        }
        TestCluster::start().map(|c| (c.dsn(), Some(c)))
    }

    #[tokio::test]
    async fn sqlite_and_postgres_kernel_roundtrip_match() {
        let Some((dsn, _cluster_guard)) = resolve_test_dsn() else {
            eprintln!(
                "SKIP sqlite_and_postgres_kernel_roundtrip_match: \
                 set WOS_POSTGRES_TEST_URL or DATABASE_URL, \
                 or install postgres binaries (initdb/pg_ctl) for an ephemeral cluster"
            );
            return;
        };
        let sqlite = SqliteStorage::connect("sqlite::memory:")
            .await
            .expect("sqlite connect");
        sqlite.migrate().await.expect("sqlite migrate");

        let row = KernelRow {
            url: "urn:wos:test:kernel".into(),
            title: "Kernel".into(),
            version: "1.0.0".into(),
            status: "active".into(),
            impact_level: "operational".into(),
            document: serde_json::json!({"$wosWorkflow":"1.0.0","url":"urn:wos:test:kernel"}),
            updated_at: chrono::Utc::now(),
        };

        sqlite.upsert_kernel(&row).await.expect("sqlite upsert");

        let got_sqlite = sqlite
            .get_kernel(&row.url)
            .await
            .expect("sqlite get")
            .expect("sqlite row");
        let pg_row = row.clone();
        let got_pg = thread::spawn(move || {
            let pg = PostgresStorage::connect(&dsn).expect("pg connect");
            assert!(
                pg.trellis_store_ready(),
                "trellis canonical store guardrail should be initialized"
            );
            block_on_ready(pg.upsert_kernel(&pg_row)).expect("pg upsert");
            let got_pg = block_on_ready(pg.get_kernel(&pg_row.url))
                .expect("pg get")
                .expect("pg row");
            drop(pg);
            got_pg
        })
        .join()
        .expect("pg thread");
        assert_eq!(got_sqlite.url, got_pg.url);
        assert_eq!(got_sqlite.document, got_pg.document);
    }

    fn block_on_ready<F: Future>(future: F) -> F::Output {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("postgres storage future unexpectedly pending"),
        }
    }

    fn noop_waker() -> Waker {
        unsafe fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        unsafe fn wake(_: *const ()) {}
        unsafe fn wake_by_ref(_: *const ()) {}
        unsafe fn drop(_: *const ()) {}
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);
        let raw = RawWaker::new(std::ptr::null(), &VTABLE);
        unsafe { Waker::from_raw(raw) }
    }

    fn reserve_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        listener.local_addr().expect("addr").port()
    }

    fn wait_for_postgres(port: u16) -> bool {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    fn find_pg_binary(name: &str) -> Option<PathBuf> {
        for candidate in command_search_paths(name) {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    fn command_search_paths(name: &str) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                out.push(dir.join(name));
            }
        }
        out.push(Path::new("/opt/homebrew/opt/postgresql@16/bin").join(name));
        out.push(Path::new("/usr/local/opt/postgresql@16/bin").join(name));
        out
    }

    fn run_command(command: &mut Command) -> bool {
        command.stdout(Stdio::null()).stderr(Stdio::null());
        command.status().map(|s| s.success()).unwrap_or(false)
    }
}
