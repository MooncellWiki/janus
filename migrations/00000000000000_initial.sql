-- Initial migration
CREATE TABLE IF NOT EXISTS health_checks (
    id SERIAL PRIMARY KEY,
    checked_at TIMESTAMP DEFAULT NOW()
);
