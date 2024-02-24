CREATE TABLE IF NOT EXISTS zone_weather (
    zone TEXT PRIMARY KEY,
    current JSONB,
    forecast JSONB,
    alert JSONB,
    last_updated_at TIMESTAMPTZ NULL DEFAULT clock_timestamp()
);
