-- Add migration script here
CREATE TABLE IF NOT EXISTS public.event_journal(
  persistence_id TEXT NOT NULL,
  sequence_number BIGINT CHECK (sequence_number >= 0) NOT NULL,
  is_deleted BOOLEAN DEFAULT FALSE NOT NULL,
  event_manifest VARCHAR(255) NOT NULL,
  event_payload BYTEA NOT NULL,
  meta_payload JSONB,
  created_at BIGINT NOT NULL,

  PRIMARY KEY(persistence_id, sequence_number)
);

CREATE TABLE IF NOT EXISTS public.snapshots(
  persistence_id TEXT NOT NULL,
  sequence_number BIGINT CHECK (sequence_number >= 0) NOT NULL,
  snapshot_manifest VARCHAR(255) NOT NULL,
  snapshot_payload BYTEA NOT NULL,
  meta_payload JSONB,
  created_at BIGINT NOT NULL,
  last_updated_at BIGINT NOT NULL,

  PRIMARY KEY(persistence_id)
);