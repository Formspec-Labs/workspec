-- Phase 1 migration: wos-runtime tables + aux columns.
--
-- `instances.runtime_aux_json` holds `RuntimeRecord`'s auxiliary fields
-- (step_results, artifacts, replay_entries) that aren't part of the
-- CaseInstance itself but live alongside it in a single atomic record.
-- The table is bumped through a rename-rebuild dance because SQLite
-- doesn't support ADD COLUMN with NOT NULL + DEFAULT reliably across
-- older connections.

ALTER TABLE instances ADD COLUMN runtime_aux_json TEXT NOT NULL DEFAULT '{}';

-- Event queue: wos-runtime keeps its own FIFO per instance. We mirror it
-- so restart survives a pending event backlog.
CREATE TABLE event_queue (
    instance_id  TEXT NOT NULL,
    seq          INTEGER NOT NULL,
    event        TEXT NOT NULL,
    actor_id     TEXT,
    data_json    TEXT,
    enqueued_at  TEXT NOT NULL,
    PRIMARY KEY (instance_id, seq)
);
CREATE INDEX idx_event_queue_instance ON event_queue(instance_id);

-- Task table denormalises wos-runtime's task artifacts for SQL queryability.
-- The authoritative store is still RuntimeRecord.artifacts; this mirror is
-- intended for a task presenter / sync path (the bundled reference presenter
-- emits Socket.IO only and does not write here yet).
CREATE TABLE tasks (
    task_id          TEXT PRIMARY KEY,
    instance_id      TEXT NOT NULL,
    task_ref         TEXT NOT NULL,
    contract_ref     TEXT,
    binding          TEXT,
    definition_url   TEXT,
    definition_version TEXT,
    assigned_actor   TEXT,
    status           TEXT NOT NULL,
    context_json     TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    updated_at       TEXT NOT NULL,
    dismissed_at     TEXT,
    dismissed_reason TEXT
);
CREATE INDEX idx_tasks_instance ON tasks(instance_id);
CREATE INDEX idx_tasks_status   ON tasks(status);

-- Agent registry (AI governance L2).
CREATE TABLE agents (
    id                TEXT PRIMARY KEY,
    workflow_url      TEXT NOT NULL,
    name              TEXT NOT NULL,
    kind              TEXT NOT NULL,
    version           TEXT NOT NULL,
    status            TEXT NOT NULL,
    autonomy          TEXT,
    confidence_floor  REAL,
    config_json       TEXT NOT NULL,
    deployment_state  TEXT NOT NULL DEFAULT 'production',
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
CREATE INDEX idx_agents_workflow ON agents(workflow_url);

-- Assurance layer: identity facts and their upgrade history.
CREATE TABLE identity_facts (
    id                   TEXT PRIMARY KEY,
    instance_id          TEXT NOT NULL,
    subject_ref          TEXT NOT NULL,
    assurance_level      TEXT NOT NULL,
    disclosure_posture   TEXT NOT NULL,
    fact_json            TEXT NOT NULL,
    upgraded_from        TEXT,
    created_at           TEXT NOT NULL
);
CREATE INDEX idx_identity_facts_instance ON identity_facts(instance_id);
CREATE INDEX idx_identity_facts_subject  ON identity_facts(subject_ref);

-- Equity report cache (L3).
CREATE TABLE equity_reports (
    id             TEXT PRIMARY KEY,
    workflow_url   TEXT NOT NULL,
    metric         TEXT NOT NULL,
    computed_at    TEXT NOT NULL,
    result_json    TEXT NOT NULL
);
CREATE INDEX idx_equity_reports_workflow ON equity_reports(workflow_url);

-- Inbound CloudEvent idempotency: dedupe by `id` within a reasonable window.
CREATE TABLE integration_inbound (
    cloud_event_id TEXT PRIMARY KEY,
    instance_id    TEXT NOT NULL,
    binding        TEXT NOT NULL,
    received_at    TEXT NOT NULL,
    payload_json   TEXT NOT NULL
);
CREATE INDEX idx_integration_inbound_instance ON integration_inbound(instance_id);
