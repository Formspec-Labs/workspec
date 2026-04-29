use async_trait::async_trait;
use postgres::{Client, NoTls};
use std::sync::{Arc, Mutex};
use wos_server_ports::audit::{AuditError, AuditResult, AuditSink, ExportEnvelope};
use wos_server_ports::storage::ProvenanceRow;

pub struct PostgresAuditSink {
    client: Arc<Mutex<Client>>,
}

impl PostgresAuditSink {
    pub fn connect(dsn: &str) -> AuditResult<Self> {
        let mut client =
            Client::connect(dsn, NoTls).map_err(|e| AuditError::Backend(e.to_string()))?;
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS provenance_audit (
                    id TEXT PRIMARY KEY,
                    instance_id TEXT NOT NULL,
                    seq BIGINT NOT NULL,
                    timestamp TIMESTAMPTZ NOT NULL,
                    tier TEXT NOT NULL,
                    payload JSONB NOT NULL,
                    hash TEXT NOT NULL,
                    previous_hash TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS export_audit (
                    case_id TEXT NOT NULL,
                    record_id TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    record JSONB NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    PRIMARY KEY (case_id, record_id)
                );",
            )
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }
}

#[async_trait]
impl AuditSink for PostgresAuditSink {
    async fn append_provenance(&self, records: &[ProvenanceRow]) -> AuditResult<()> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| AuditError::Backend("audit client mutex poisoned".into()))?;
        let mut tx = client
            .transaction()
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        for row in records {
            if row
                .payload
                .get("__injectAuditFailure")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return Err(AuditError::Backend(
                    "injected failure before audit transaction commit".into(),
                ));
            }
            let payload = serde_json::to_string(&row.payload)
                .map_err(|e| AuditError::Backend(e.to_string()))?;
            let ts = row.timestamp.to_rfc3339();
            tx.execute(
                "INSERT INTO provenance_audit
                (id, instance_id, seq, timestamp, tier, payload, hash, previous_hash)
                VALUES ($1,$2,$3,$4::timestamptz,$5,$6::jsonb,$7,$8)
                ON CONFLICT (id) DO NOTHING",
                &[
                    &row.id,
                    &row.instance_id,
                    &row.seq,
                    &ts,
                    &row.tier,
                    &payload,
                    &row.hash,
                    &row.previous_hash,
                ],
            )
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn append_export(&self, envelope: ExportEnvelope) -> AuditResult<()> {
        let mut client = self
            .client
            .lock()
            .map_err(|_| AuditError::Backend("audit client mutex poisoned".into()))?;
        let record = serde_json::to_string(&envelope.record)
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        let mut tx = client
            .transaction()
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        tx.execute(
                "INSERT INTO export_audit (case_id, record_id, event_type, record)
                VALUES ($1,$2,$3,$4::jsonb)
                ON CONFLICT (case_id, record_id) DO NOTHING",
                &[
                    &envelope.case_id,
                    &envelope.record_id,
                    &envelope.event_type,
                    &record,
                ],
            ).map_err(|e| AuditError::Backend(e.to_string()))?;
        tx.commit()
            .map_err(|e| AuditError::Backend(e.to_string()))?;
        Ok(())
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
    use wos_server_ports::storage::ProvenanceRow;

    #[tokio::test(flavor = "current_thread")]
    async fn failure_injection_rolls_back_batch() {
        let Some(cluster) = TestCluster::start() else {
            return;
        };
        let sink = PostgresAuditSink::connect(&cluster.dsn()).expect("sink connect");
        let mut client = postgres::Client::connect(&cluster.dsn(), postgres::NoTls).expect("probe");

        let now = chrono::Utc::now();
        let ok = ProvenanceRow {
            id: "p1".into(),
            instance_id: "i1".into(),
            seq: 1,
            timestamp: now,
            tier: "wos.t1".into(),
            payload: serde_json::json!({"ok": true}),
            hash: "h1".into(),
            previous_hash: "h0".into(),
        };
        let fail = ProvenanceRow {
            id: "p2".into(),
            instance_id: "i1".into(),
            seq: 2,
            timestamp: now,
            tier: "wos.t1".into(),
            payload: serde_json::json!({"__injectAuditFailure": true}),
            hash: "h2".into(),
            previous_hash: "h1".into(),
        };
        let err = sink
            .append_provenance(&[ok, fail])
            .await
            .expect_err("failure injection should fail");
        assert!(err.to_string().contains("injected failure"));

        let row = client
            .query_one("SELECT COUNT(*) AS n FROM provenance_audit", &[])
            .expect("count rows");
        let n: i64 = row.get("n");
        assert_eq!(n, 0, "batch should fully rollback on injected failure");
    }

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
            Some(Self { temp_dir, port, pg_ctl })
        }

        fn dsn(&self) -> String {
            format!("host=127.0.0.1 port={} user=postgres dbname=postgres", self.port)
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
