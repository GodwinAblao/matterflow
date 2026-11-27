ALTER TABLE matterflow.follow_ups ADD COLUMN version INTEGER NOT NULL DEFAULT 0 CHECK (version >= 0);
