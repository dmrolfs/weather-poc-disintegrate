CREATE TABLE IF NOT EXISTS update_weather_history (
    update_id TEXT PRIMARY KEY,
    state TEXT,
    update_statuses JSONB,
    last_updated_at TIMESTAMPTZ NULL DEFAULT clock_timestamp()
);
