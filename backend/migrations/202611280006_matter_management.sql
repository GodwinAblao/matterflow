ALTER TABLE matterflow.matters ADD COLUMN deleted_at TIMESTAMPTZ;
CREATE INDEX matters_visible_idx ON matterflow.matters(workspace_id) WHERE deleted_at IS NULL;
