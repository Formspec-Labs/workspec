-- Durable intake-acceptance receipts.
--
-- wos-runtime emits IntakeRecord via create_intake_record / save_intake_record
-- on the RuntimeStore trait. The reference server previously held these in an
-- in-memory map, so process restart silently dropped accepted-but-not-applied
-- intake work. This table persists the full record (request + outcome +
-- provenance_log + status) keyed by (binding, intake_id). JSON payload is
-- round-trippable through serde on wos_runtime::intake types.

CREATE TABLE intake_records (
    binding        TEXT NOT NULL,
    intake_id      TEXT NOT NULL,
    status         TEXT NOT NULL,
    record_json    TEXT NOT NULL,
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL,
    PRIMARY KEY (binding, intake_id)
);
CREATE INDEX idx_intake_records_status ON intake_records(status);
