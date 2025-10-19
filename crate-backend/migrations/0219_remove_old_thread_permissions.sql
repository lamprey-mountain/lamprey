-- Remove unused permissions from data
UPDATE role
SET permissions = COALESCE((
    SELECT array_agg(p)
    FROM unnest(permissions) as p
    WHERE p::text NOT IN (
        'ThreadArchive',
        'ThreadCreateChat',
        'ThreadCreateForum',
        'ThreadCreateVoice',
        'ThreadForward',
        'ThreadRemove',
        'ThreadPublish'
    )
), '{}'::permission[]);

UPDATE permission_overwrite
SET
    allow = COALESCE((
        SELECT jsonb_agg(elem)
        FROM jsonb_array_elements_text(allow) AS elem
        WHERE elem NOT IN (
            'ThreadArchive',
            'ThreadCreateChat',
            'ThreadCreateForum',
            'ThreadCreateVoice',
            'ThreadForward',
            'ThreadRemove',
            'ThreadPublish'
        )
    ), '[]'::jsonb),
    deny = COALESCE((
        SELECT jsonb_agg(elem)
        FROM jsonb_array_elements_text(deny) AS elem
        WHERE elem NOT IN (
            'ThreadArchive',
            'ThreadCreateChat',
            'ThreadCreateForum',
            'ThreadCreateVoice',
            'ThreadForward',
            'ThreadRemove',
            'ThreadPublish'
        )
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
    'MessageRemove',
    'MessageEmbeds',
    'MessageMassMention',
    'MessageMove',
    'MessagePin',
    'MemberNickname',
    'MemberTimeout',
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
    'ChannelEdit',
    'ChannelManage',
    'ThreadCreatePrivate',
    'ThreadCreatePublic',
    'ThreadManage',
    'ThreadEdit',
    'ThreadLock',
    'ViewChannel',
    'ViewAuditLog',
    'VoiceConnect',
    'VoiceDeafen',
    'VoiceDisconnect',
    'VoiceMove',
    'VoiceMute',
    'VoicePriority',
    'VoiceSpeak',
    'VoiceVideo',
    'CalendarEventManage'
);

ALTER TABLE role ALTER COLUMN permissions TYPE permission[] USING permissions::text[]::permission[];

DROP TYPE permission_old;
