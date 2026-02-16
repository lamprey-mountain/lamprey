-- Remove unused permissions from data
WITH updated_roles AS (
    SELECT
        r.id,
        (SELECT array_agg(p) FROM unnest(r.permissions) as p WHERE p::text NOT LIKE 'Unused%') as new_permissions
    FROM
        role r
)
UPDATE role
SET permissions = COALESCE(updated_roles.new_permissions, '{}'::permission[])
FROM updated_roles
WHERE role.id = updated_roles.id;

UPDATE permission_overwrite
SET
    allow = COALESCE((
        SELECT jsonb_agg(elem)
        FROM jsonb_array_elements_text(allow) AS elem
        WHERE elem NOT LIKE 'Unused%'
    ), '[]'::jsonb),
    deny = COALESCE((
        SELECT jsonb_agg(elem)
        FROM jsonb_array_elements_text(deny) AS elem
        WHERE elem NOT LIKE 'Unused%'
    ), '[]'::jsonb);

-- Recreate the permission enum type without unused values
ALTER TYPE permission RENAME TO permission_old;

CREATE TYPE permission AS ENUM (
    'Admin',
    'IntegrationsManage',
    'EmojiManage',
    'EmojiUseExternal',
    'InviteCreate',
    'InviteManage',
    'MemberBan',
    'MemberBridge',
    'MemberKick',
    'MemberNicknameManage',
    'MessageAttachments',
    'MessageCreate',
    'MessageDelete',
    'MessageEmbeds',
    'MessageMassMention',
    'MessageMove',
    'MessagePin',
    'MemberNickname',
    'ReactionAdd',
    'ReactionPurge',
    'RoleApply',
    'RoleManage',
    'RoomManage',
    'ServerMetrics',
    'ServerOversee',
    'ServerReports',
    'TagApply',
    'TagManage',
    'ThreadArchive',
    'ThreadCreateChat',
    'ThreadCreateForum',
    'ThreadCreatePrivate',
    'ThreadCreatePublic',
    'ThreadCreateVoice',
    'ThreadRemove',
    'ThreadEdit',
    'ThreadForward',
    'ThreadLock',
    'ThreadManage',
    'ThreadPublish',
    'View',
    'ViewAuditLog',
    'VoiceConnect',
    'VoiceDeafen',
    'VoiceDisconnect',
    'VoiceMove',
    'VoiceMute',
    'VoicePriority',
    'VoiceSpeak',
    'VoiceVideo'
);

ALTER TABLE role ALTER COLUMN permissions TYPE permission[] USING permissions::text[]::permission[];

DROP TYPE permission_old;
