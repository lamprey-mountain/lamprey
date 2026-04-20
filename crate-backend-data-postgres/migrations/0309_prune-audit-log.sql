delete from audit_log where data->>'type' in ('ReactionPurge', 'PermissionOverwriteSet');
