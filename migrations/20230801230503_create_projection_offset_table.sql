-- Add migration script here
CREATE TABLE IF NOT EXISTS public.projection_offset(
  projection_id VARCHAR(255) NOT NULL,
  persistence_id VARCHAR(255) NOT NULL,
  current_offset BIGINT NOT NULL,
  last_updated_at BIGINT NOT NULL,

  PRIMARY KEY(projection_id, persistence_id)
);

CREATE INDEX IF NOT EXISTS projection_id_index ON projection_offset (projection_id);

--CREATE TABLE IF NOT EXISTS projection_management {
--  projection_name VARCHAR(255) NOT NULL,
--  projection_key VARCHAR(255) NOT NULL,
--  paused BOOLEAN NOT NULL,
--  last_updated BIGINT NOT NULL,
--  PRIMARY KEY(projection_name, projection_key)
--);
