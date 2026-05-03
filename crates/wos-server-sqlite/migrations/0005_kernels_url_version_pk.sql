-- Allow multiple kernel rows per workflow URL (one per definitionVersion) so
-- reference-server bundle resolution can satisfy ADR 0083 cross-version migrate.
PRAGMA foreign_keys = OFF;

CREATE TABLE kernels_new (
  url          TEXT NOT NULL,
  version      TEXT NOT NULL,
  title        TEXT NOT NULL,
  status       TEXT NOT NULL,
  impact_level TEXT NOT NULL,
  document     TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  PRIMARY KEY (url, version)
);

INSERT INTO kernels_new (url, version, title, status, impact_level, document, updated_at)
SELECT url, version, title, status, impact_level, document, updated_at FROM kernels;

DROP TABLE kernels;
ALTER TABLE kernels_new RENAME TO kernels;

PRAGMA foreign_keys = ON;
