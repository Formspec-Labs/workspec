-- Initial schema for wos-server.
-- All JSON columns use SQLite's JSON1 storage (TEXT with JSON functions).

CREATE TABLE kernels (
  url          TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  version      TEXT NOT NULL,
  status       TEXT NOT NULL,
  impact_level TEXT NOT NULL,
  document     TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);

CREATE TABLE instances (
  instance_id        TEXT PRIMARY KEY,
  definition_url     TEXT NOT NULL,
  definition_version TEXT NOT NULL,
  status             TEXT NOT NULL,
  configuration      TEXT NOT NULL,
  case_state         TEXT NOT NULL,
  active_tasks       TEXT NOT NULL,
  timers             TEXT NOT NULL,
  governance_state   TEXT,
  impact_level       TEXT NOT NULL,
  created_at         TEXT NOT NULL,
  updated_at         TEXT NOT NULL
);

CREATE INDEX idx_instances_status        ON instances(status);
CREATE INDEX idx_instances_impact_level  ON instances(impact_level);
CREATE INDEX idx_instances_definition    ON instances(definition_url);
CREATE INDEX idx_instances_created_desc  ON instances(created_at DESC);

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
