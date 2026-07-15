-- Migration: Edge Builder & Edge Analyzer tables
-- Creates isolated tables for strategy configuration persistence and cached analytics.
-- This migration is purely additive and does not alter any existing tables.

CREATE TABLE IF NOT EXISTS saved_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    pair_key        TEXT NOT NULL,
    description     TEXT,
    config_payload  TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS edge_analytics_cache (
    edge_id             INTEGER PRIMARY KEY,
    historical_metrics  TEXT NOT NULL,
    monte_carlo_paths   TEXT NOT NULL,
    bootstrap_results   TEXT NOT NULL,
    generated_at        TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (edge_id) REFERENCES saved_edges(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_saved_edges_name ON saved_edges(name);
CREATE INDEX IF NOT EXISTS idx_edge_analytics_cache_edge_id ON edge_analytics_cache(edge_id);
