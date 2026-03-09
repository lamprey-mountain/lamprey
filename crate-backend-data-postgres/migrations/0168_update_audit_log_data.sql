update audit_log set data = jsonb_build_object(
    'type', data->'type',
    'metadata', data
);

update audit_log set data = jsonb_build_object(
    'type', 'ThreadOverwriteSet',
    'metadata', jsonb_build_object(
        'thread_id', data->'metadata'->>'thread_id',
        'overwrite_id', data->'metadata'->>'overwrite_id',
        'type', data->'metadata'->>'ty',
        'changes', jsonb_build_array(
            jsonb_build_object(
                'key', 'allow',
                'old', null,
                'new', data->'metadata'->'allow'
            ),
            jsonb_build_object(
                'key', 'deny',
                'old', null,
                'new', data->'metadata'->'deny'
            )
        )
    )
)
where data->>'type' = 'ThreadOverwriteSet';
