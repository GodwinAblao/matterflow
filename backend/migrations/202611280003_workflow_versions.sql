ALTER TABLE matterflow.matters ADD COLUMN version INTEGER NOT NULL DEFAULT 0 CHECK (version >= 0);
ALTER TABLE matterflow.document_requirements ADD COLUMN version INTEGER NOT NULL DEFAULT 0 CHECK (version >= 0);
