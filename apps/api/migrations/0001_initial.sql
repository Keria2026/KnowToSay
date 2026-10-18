CREATE TABLE workspace (id INTEGER PRIMARY KEY CHECK (id = 1), json TEXT NOT NULL);
CREATE TABLE official_nodes (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE draft_nodes (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE personal_nodes (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE personal_dependencies (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE official_materials (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE draft_materials (id TEXT PRIMARY KEY, json TEXT NOT NULL);
CREATE TABLE releases (version INTEGER PRIMARY KEY, json TEXT NOT NULL);
CREATE INDEX release_version ON releases(version DESC);
