CREATE TABLE IF NOT EXISTS kernels (
    url TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    version TEXT NOT NULL,
    status TEXT NOT NULL,
    impact_level TEXT NOT NULL,
    document TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS instances (
    instance_id TEXT PRIMARY KEY,
    definition_url TEXT NOT NULL,
    definition_version TEXT NOT NULL,
    status TEXT NOT NULL,
    impact_level TEXT NOT NULL,
    instance_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS provenance (
    id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    timestamp TEXT NOT NULL,
    tier TEXT NOT NULL,
    payload TEXT NOT NULL,
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
    start_date TEXT NOT NULL,
    end_date TEXT,
    status TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    avatar TEXT,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_users_email_lower ON users(lower(email));

CREATE TABLE IF NOT EXISTS sessions (
    jti TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0
);
