-- Fictional administrative examples, not prescribed legal requirements.
DO $$
DECLARE
    template_id UUID := md5('matterflow:portfolio-v1')::uuid;
    staff_a UUID := md5('matterflow:staff-a')::uuid;
    staff_b UUID := md5('matterflow:staff-b')::uuid;
    reviewer UUID := md5('matterflow:reviewer')::uuid;
BEGIN
    PERFORM pg_advisory_xact_lock(74621001);
    IF EXISTS (SELECT 1 FROM matterflow.workspaces WHERE template_key = 'portfolio-v1') THEN
        RETURN;
    END IF;
    INSERT INTO matterflow.workspaces(id, name, is_template, template_key)
        VALUES (template_id, 'Fictional portfolio examples', TRUE, 'portfolio-v1');
    INSERT INTO matterflow.app_users(id, workspace_id, display_name, role) VALUES
        (staff_a, template_id, 'Alex Santos (Demo)', 'staff'),
        (staff_b, template_id, 'Sam Rivera (Demo)', 'staff'),
        (reviewer, template_id, 'Jamie Cruz (Demo)', 'reviewer');

    INSERT INTO matterflow.matters(id, workspace_id, reference, title, client_label,
        service_category, status, assigned_staff_id, assigned_reviewer_id,
        created_at, updated_at, first_ready_at, closed_at)
    SELECT md5('matterflow:matter:' || n)::uuid, template_id,
        'MF-' || lpad(n::text, 3, '0'),
        (ARRAY['Property document review', 'Lease review preparation', 'Business registration intake',
            'Contract review preparation', 'Family matter intake', 'Property transfer coordination',
            'Client document follow-up', 'Corporate records review', 'Consultation preparation',
            'Title document coordination', 'Business agreement intake', 'Remote client document review'])[n],
        'Fictional Client ' || lpad(n::text, 2, '0'),
        (ARRAY['property', 'property', 'business', 'business', 'family', 'property',
            'other', 'business', 'other', 'property', 'business', 'property'])[n],
        (ARRAY['intake', 'collecting_documents', 'under_review', 'ready', 'closed'])[(n - 1) % 5 + 1],
        CASE WHEN n % 2 = 1 THEN staff_a ELSE staff_b END, reviewer,
        now() - (n + 8) * interval '1 day', now(),
        CASE WHEN (n - 1) % 5 IN (3, 4) THEN now() - (n + 3) * interval '1 day' END,
        CASE WHEN (n - 1) % 5 = 4 THEN now() - (n + 1) * interval '1 day' END
    FROM generate_series(1, 12) AS n;

    INSERT INTO matterflow.document_requirements(id, workspace_id, matter_id, title,
        is_required, status, requested_at, received_at, reviewed_at, reviewed_by, correction_note)
    SELECT md5(m.id::text || ':document:' || d)::uuid, template_id, m.id,
        (ARRAY['Client-provided timeline', 'Supporting document copy', 'Contact details sheet', 'Additional reference notes'])[d],
        d <> 4, s.status, m.created_at + interval '1 day',
        CASE WHEN s.status <> 'missing' THEN m.created_at + interval '2 days' END,
        CASE WHEN s.status IN ('accepted', 'needs_correction') THEN m.created_at + interval '3 days' END,
        CASE WHEN s.status IN ('accepted', 'needs_correction') THEN reviewer END,
        CASE WHEN s.status = 'needs_correction' THEN 'Demo: replace the unreadable sample copy.' END
    FROM matterflow.matters m CROSS JOIN generate_series(1, 4) AS d
    CROSS JOIN LATERAL (SELECT CASE
        WHEN m.status IN ('ready', 'closed') THEN 'accepted'
        WHEN m.status = 'intake' THEN 'missing'
        WHEN d = 1 THEN 'accepted'
        WHEN d = 2 THEN 'received'
        WHEN d = 3 THEN 'needs_correction'
        ELSE 'missing' END AS status) s
    WHERE m.workspace_id = template_id;

    INSERT INTO matterflow.follow_ups(id, workspace_id, matter_id, requirement_id,
        assigned_to, title, due_on, created_at, completed_at)
    SELECT md5(m.id::text || ':follow-up')::uuid, template_id, m.id,
        md5(m.id::text || ':document:3')::uuid, m.assigned_staff_id,
        'Follow up on supporting requirements',
        (now() AT TIME ZONE 'Asia/Manila')::date + ((right(m.reference, 3)::int % 5) - 2),
        m.created_at + interval '1 day',
        CASE WHEN m.status IN ('ready', 'closed') THEN m.created_at + interval '4 days' END
    FROM matterflow.matters m WHERE m.workspace_id = template_id;

    INSERT INTO matterflow.activity_events(workspace_id, matter_id, actor_id, event_type, details, occurred_at)
    SELECT template_id, id, assigned_staff_id, 'matter.opened',
        '{"synthetic":true,"note":"Fictional portfolio example"}'::jsonb, created_at
    FROM matterflow.matters WHERE workspace_id = template_id;
END;
$$;
