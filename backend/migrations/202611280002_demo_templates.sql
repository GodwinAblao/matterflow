ALTER TABLE matterflow.workspaces ADD COLUMN template_key TEXT UNIQUE;
ALTER TABLE matterflow.workspaces ADD CONSTRAINT workspace_template_key
    CHECK (is_template = (template_key IS NOT NULL));
CREATE UNIQUE INDEX demo_sessions_workspace_idx ON matterflow.demo_sessions(workspace_id);
