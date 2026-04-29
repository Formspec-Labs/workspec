-- Initial schema for wos-server. All JSON columns are TEXT with application-
-- managed serde_json round-tripping (no JSON1 extension assumed).
--
-- Subsequent migrations (read these together when reasoning about the
-- runtime schema — this file is *not* the full picture):
--   0002_runtime_tables.sql:
--     - `instances.runtime_aux_json` column (carries StepResult / TaskArtifact /
--       ReplayEntry aux state alongside the canonical `instance_json`)
--     - `agents`, `tasks`, `identity_facts`, `integration_inbound` tables
--   0003_intake_records.sql:
--     - `intake_records` table (Formspec→WOS handoff per ADR 0073)
--   0004_user_auth_epoch.sql:
--     - `users.auth_epoch` column (monotonic counter for session invalidation;
--       bumped on global logout and password rotation — see
--       `set_user_password_hash` / `bump_user_auth_epoch`)
--
-- The Postgres adapter (`crates/wos-server-postgres/src/lib.rs::migrate`)
-- inlines the union of all four migrations as a single `CREATE TABLE IF NOT
-- EXISTS` batch. Schema parity between the two adapters is load-bearing — see
-- `crates/wos-server/PARITY.md`.

CREATE TABLE kernels (
  url          TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  version      TEXT NOT NULL,
  status       TEXT NOT NULL,
  impact_level TEXT NOT NULL,
  document     TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

-- `instance_json` holds the full wos-core `CaseInstance` document so that
-- the server can round-trip through `Evaluator::from_instance` without
-- losing any runtime bookkeeping (history_store, fired_milestones,
-- pending_events, compensation logs, volume_counters, extensions, ...).
-- The other columns are denormalized indexes populated from
-- `instance_json` at write time — treat them as read-only search hints.
CREATE TABLE instances (
  instance_id        TEXT PRIMARY KEY,
  definition_url     TEXT NOT NULL,
  definition_version TEXT NOT NULL,
  status             TEXT NOT NULL,
  impact_level       TEXT NOT NULL,
  instance_json      TEXT NOT NULL,
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL
);

CREATE INDEX idx_instances_status       ON instances(status);
CREATE INDEX idx_instances_impact_level ON instances(impact_level);
CREATE INDEX idx_instances_definition   ON instances(definition_url);
CREATE INDEX idx_instances_created_desc ON instances(created_at DESC);

CREATE TABLE provenance (
  id            TEXT PRIMARY KEY,
  instance_id   TEXT NOT NULL,
  seq           INTEGER NOT NULL,
  timestamp     TEXT NOT NULL,
  tier          TEXT NOT NULL,
  payload       TEXT NOT NULL,
  hash          TEXT NOT NULL,
  previous_hash TEXT NOT NULL,
  UNIQUE(instance_id, seq)
);

CREATE INDEX idx_provenance_instance ON provenance(instance_id, seq);

CREATE TABLE delegations (
  id               TEXT PRIMARY KEY,
  workflow_url     TEXT NOT NULL,
  delegator        TEXT NOT NULL,
  delegate         TEXT NOT NULL,
  scope            TEXT NOT NULL,
  authority        TEXT,
  legal_instrument TEXT,
  start_date       TEXT NOT NULL,
  end_date         TEXT,
  status           TEXT NOT NULL
);

CREATE INDEX idx_delegations_workflow ON delegations(workflow_url);

CREATE TABLE users (
  id            TEXT PRIMARY KEY,
  email         TEXT NOT NULL,
  name          TEXT NOT NULL,
  role          TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  avatar        TEXT,
  created_at    TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_users_email_lower ON users(lower(email));

CREATE TABLE sessions (
  jti          TEXT PRIMARY KEY,
  user_id      TEXT NOT NULL,
  expires_at   TEXT NOT NULL,
  revoked      INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
