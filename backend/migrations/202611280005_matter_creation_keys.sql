ALTER TABLE matterflow.matters ADD COLUMN creation_key UUID;
ALTER TABLE matterflow.matters ADD COLUMN creation_fingerprint TEXT;
ALTER TABLE matterflow.matters ADD CONSTRAINT matter_creation_request
    CHECK ((creation_key IS NULL AND creation_fingerprint IS NULL)
        OR (creation_key IS NOT NULL AND creation_fingerprint IS NOT NULL AND creation_fingerprint ~ '^[0-9a-f]{64}$'));
ALTER TABLE matterflow.matters ADD CONSTRAINT matter_creation_key UNIQUE (workspace_id,creation_key);
