-- Monotonic per-user counter: incremented on logout so in-flight refresh
-- cannot mint tokens after the server has invalidated the login generation.
ALTER TABLE users ADD COLUMN auth_epoch INTEGER NOT NULL DEFAULT 0;
