-- Initial schema: no real clients or demonstration records are inserted.
CREATE SCHEMA matterflow;

CREATE TABLE matterflow.workspaces (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL CHECK (length(btrim(name)) BETWEEN 1 AND 120),
    is_template BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    CHECK (expires_at IS NULL OR expires_at > created_at)
);

CREATE TABLE matterflow.app_users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL CHECK (length(btrim(display_name)) BETWEEN 1 AND 120),
    role TEXT NOT NULL CHECK (role IN ('staff', 'reviewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, id)
);

CREATE TABLE matterflow.demo_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    token_hash TEXT NOT NULL UNIQUE CHECK (token_hash ~ '^[0-9a-f]{64}$'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (workspace_id, user_id) REFERENCES matterflow.app_users(workspace_id, id),
    CHECK (expires_at > created_at)
);

CREATE TABLE matterflow.matters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    reference TEXT NOT NULL CHECK (length(btrim(reference)) BETWEEN 1 AND 40),
    title TEXT NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    client_label TEXT NOT NULL CHECK (length(btrim(client_label)) BETWEEN 1 AND 120),
    service_category TEXT NOT NULL CHECK (service_category IN
        ('property', 'business', 'family', 'criminal', 'other')),
    status TEXT NOT NULL DEFAULT 'intake' CHECK (status IN
        ('intake', 'collecting_documents', 'under_review', 'ready', 'closed')),
    assigned_staff_id UUID,
    assigned_reviewer_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    first_ready_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    UNIQUE (workspace_id, id),
    UNIQUE (workspace_id, reference),
    FOREIGN KEY (workspace_id, assigned_staff_id) REFERENCES matterflow.app_users(workspace_id, id),
    FOREIGN KEY (workspace_id, assigned_reviewer_id) REFERENCES matterflow.app_users(workspace_id, id),
    CHECK (updated_at >= created_at),
    CHECK (first_ready_at IS NULL OR first_ready_at >= created_at),
    CHECK (status <> 'ready' OR first_ready_at IS NOT NULL),
    CHECK ((status = 'closed') = (closed_at IS NOT NULL)),
    CHECK (closed_at IS NULL OR closed_at >= created_at)
);

CREATE TABLE matterflow.document_requirements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    matter_id UUID NOT NULL,
    title TEXT NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    is_required BOOLEAN NOT NULL DEFAULT TRUE,
    status TEXT NOT NULL DEFAULT 'missing' CHECK (status IN
        ('missing', 'received', 'needs_correction', 'accepted')),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    received_at TIMESTAMPTZ,
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID,
    correction_note TEXT,
    UNIQUE (workspace_id, matter_id, id),
    FOREIGN KEY (workspace_id, matter_id) REFERENCES matterflow.matters(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, reviewed_by) REFERENCES matterflow.app_users(workspace_id, id),
    CHECK (received_at IS NULL OR received_at >= requested_at),
    CHECK (reviewed_at IS NULL OR reviewed_at >= received_at),
    CONSTRAINT document_review_state CHECK (
        (status = 'missing' AND received_at IS NULL AND reviewed_at IS NULL AND reviewed_by IS NULL AND correction_note IS NULL)
        OR (status = 'received' AND received_at IS NOT NULL AND reviewed_at IS NULL AND reviewed_by IS NULL AND correction_note IS NULL)
        OR (status = 'needs_correction' AND received_at IS NOT NULL AND reviewed_at IS NOT NULL AND reviewed_by IS NOT NULL
            AND correction_note IS NOT NULL AND length(btrim(correction_note)) > 0)
        OR (status = 'accepted' AND received_at IS NOT NULL AND reviewed_at IS NOT NULL AND reviewed_by IS NOT NULL AND correction_note IS NULL)
    )
);

CREATE TABLE matterflow.follow_ups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    matter_id UUID NOT NULL,
    requirement_id UUID,
    assigned_to UUID,
    title TEXT NOT NULL CHECK (length(btrim(title)) BETWEEN 1 AND 200),
    due_on DATE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at TIMESTAMPTZ,
    FOREIGN KEY (workspace_id, matter_id) REFERENCES matterflow.matters(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, matter_id, requirement_id)
        REFERENCES matterflow.document_requirements(workspace_id, matter_id, id),
    FOREIGN KEY (workspace_id, assigned_to) REFERENCES matterflow.app_users(workspace_id, id),
    CHECK (completed_at IS NULL OR completed_at >= created_at)
);

CREATE TABLE matterflow.activity_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id UUID NOT NULL REFERENCES matterflow.workspaces(id) ON DELETE CASCADE,
    matter_id UUID NOT NULL,
    actor_id UUID,
    event_type TEXT NOT NULL CHECK (length(btrim(event_type)) BETWEEN 1 AND 80),
    details JSONB NOT NULL DEFAULT '{}'::jsonb CHECK (jsonb_typeof(details) = 'object'),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (workspace_id, matter_id) REFERENCES matterflow.matters(workspace_id, id) ON DELETE CASCADE,
    FOREIGN KEY (workspace_id, actor_id) REFERENCES matterflow.app_users(workspace_id, id)
);

CREATE INDEX workspaces_expiry_idx ON matterflow.workspaces(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX demo_sessions_expiry_idx ON matterflow.demo_sessions(expires_at);
CREATE INDEX matters_stage_idx ON matterflow.matters(workspace_id, status, created_at);
CREATE INDEX matters_staff_idx ON matterflow.matters(workspace_id, assigned_staff_id) WHERE status <> 'closed';
CREATE INDEX document_requirements_pending_idx ON matterflow.document_requirements(workspace_id, matter_id, requested_at)
    WHERE is_required AND status <> 'accepted';
CREATE INDEX document_requirements_reviewer_idx ON matterflow.document_requirements(workspace_id, reviewed_by);
CREATE INDEX follow_ups_due_idx ON matterflow.follow_ups(workspace_id, due_on) WHERE completed_at IS NULL;
CREATE INDEX follow_ups_matter_idx ON matterflow.follow_ups(workspace_id, matter_id);
CREATE INDEX follow_ups_assignee_idx ON matterflow.follow_ups(workspace_id, assigned_to);
CREATE INDEX activity_events_timeline_idx ON matterflow.activity_events(workspace_id, matter_id, occurred_at, id);
