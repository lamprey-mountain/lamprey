ALTER TYPE permission RENAME TO permission_old;

CREATE TYPE permission AS ENUM (
    'Admin',
    'ApplicationCreate',
    'ApplicationManage',
    'IntegrationsManage',
    'IntegrationsBridge',
    'EmojiManage',
    'EmojiUseExternal',
    'InviteCreate',
    'InviteManage',
    'MemberBan',
    'MemberKick',
    'MemberNickname',
    'MemberNicknameManage',
    'MemberTimeout',
    'MessageAttachments',
    'MessageCreate',
    'MessageCreateThread',
    'MessageDelete',
    'MessageEmbeds',
    'MessageMassMention',
    'MessageMove',
    'MessagePin',
    'MessageRemove',
    'ReactionAdd',
    'ReactionManage',
    'RoleApply',
    'RoleManage',
    'RoomEdit',
    'RoomManage',
    'ServerMaintenance',
    'ServerMetrics',
    'ServerOversee',
    'ChannelSlowmodeBypass',
    'ChannelEdit',
    'ChannelManage',
    'ThreadCreatePrivate',
    'ThreadCreatePublic',
    'ThreadManage',
    'ThreadEdit',
    'ChannelView',
    'AuditLogView',
    'AnalyticsView',
    'VoiceDeafen',
    'VoiceMove',
    'VoiceMute',
    'VoicePriority',
    'VoiceSpeak',
    'VoiceVideo',
    'VoiceVad',
    'VoiceRequest',
    'VoiceBroadcast',
    'CalendarEventCreate',
    'CalendarEventRsvp',
    'CalendarEventManage',
    'DocumentCreate',
    'DocumentEdit',
    'DocumentComment',
    'RoomCreate',
    'UserManage',
    'UserManageSelf',
    'UserProfileSelf',
    'DmCreate',
    'FriendCreate',
    'RoomJoin',
    'CallUpdate',
    'RoomJoinForce'
);

CREATE OR REPLACE FUNCTION convert_permission(old_perm text) RETURNS permission AS $$
BEGIN
    RETURN CASE old_perm
        WHEN 'Admin' THEN 'Admin'
        WHEN 'ApplicationCreate' THEN 'ApplicationCreate'
        WHEN 'ApplicationManage' THEN 'ApplicationManage'
        WHEN 'IntegrationsManage' THEN 'IntegrationsManage'
        WHEN 'MemberBridge' THEN 'IntegrationsBridge'
        WHEN 'EmojiManage' THEN 'EmojiManage'
        WHEN 'EmojiUseExternal' THEN 'EmojiUseExternal'
        WHEN 'InviteCreate' THEN 'InviteCreate'
        WHEN 'InviteManage' THEN 'InviteManage'
        WHEN 'MemberBan' THEN 'MemberBan'
        WHEN 'MemberKick' THEN 'MemberKick'
        WHEN 'MemberNickname' THEN 'MemberNickname'
        WHEN 'MemberNicknameManage' THEN 'MemberNicknameManage'
        WHEN 'MemberTimeout' THEN 'MemberTimeout'
        WHEN 'MessageAttachments' THEN 'MessageAttachments'
        WHEN 'MessageCreate' THEN 'MessageCreate'
        WHEN 'MessageCreateThread' THEN 'MessageCreateThread'
        WHEN 'MessageDelete' THEN 'MessageDelete'
        WHEN 'MessageEmbeds' THEN 'MessageEmbeds'
        WHEN 'MessageMassMention' THEN 'MessageMassMention'
        WHEN 'MessageMove' THEN 'MessageMove'
        WHEN 'MessagePin' THEN 'MessagePin'
        WHEN 'MessageRemove' THEN 'MessageRemove'
        WHEN 'ReactionAdd' THEN 'ReactionAdd'
        WHEN 'ReactionPurge' THEN 'ReactionManage'
        WHEN 'RoleApply' THEN 'RoleApply'
        WHEN 'RoleManage' THEN 'RoleManage'
        WHEN 'RoomManage' THEN 'RoomEdit'
        WHEN 'RoomManageServer' THEN 'RoomManage'
        WHEN 'ServerMaintenance' THEN 'ServerMaintenance'
        WHEN 'ServerMetrics' THEN 'ServerMetrics'
        WHEN 'ServerOversee' THEN 'ServerOversee'
        WHEN 'BypassSlowmode' THEN 'ChannelSlowmodeBypass'
        WHEN 'ChannelEdit' THEN 'ChannelEdit'
        WHEN 'ChannelManage' THEN 'ChannelManage'
        WHEN 'ThreadCreatePrivate' THEN 'ThreadCreatePrivate'
        WHEN 'ThreadCreatePublic' THEN 'ThreadCreatePublic'
        WHEN 'ThreadManage' THEN 'ThreadManage'
        WHEN 'ThreadEdit' THEN 'ThreadEdit'
        WHEN 'ViewChannel' THEN 'ChannelView'
        WHEN 'ViewAuditLog' THEN 'AuditLogView'
        WHEN 'ViewAnalytics' THEN 'AnalyticsView'
        WHEN 'VoiceDeafen' THEN 'VoiceDeafen'
        WHEN 'VoiceDisconnect' THEN 'VoiceMove'
        WHEN 'VoiceMove' THEN 'VoiceMove'
        WHEN 'VoiceMute' THEN 'VoiceMute'
        WHEN 'VoicePriority' THEN 'VoicePriority'
        WHEN 'VoiceSpeak' THEN 'VoiceSpeak'
        WHEN 'VoiceVideo' THEN 'VoiceVideo'
        WHEN 'VoiceVad' THEN 'VoiceVad'
        WHEN 'VoiceRequest' THEN 'VoiceRequest'
        WHEN 'VoiceBroadcast' THEN 'VoiceBroadcast'
        WHEN 'CalendarEventCreate' THEN 'CalendarEventCreate'
        WHEN 'CalendarEventRsvp' THEN 'CalendarEventRsvp'
        WHEN 'CalendarEventManage' THEN 'CalendarEventManage'
        WHEN 'DocumentCreate' THEN 'DocumentCreate'
        WHEN 'DocumentEdit' THEN 'DocumentEdit'
        WHEN 'DocumentComment' THEN 'DocumentComment'
        WHEN 'RoomCreate' THEN 'RoomCreate'
        WHEN 'UserManage' THEN 'UserManage'
        WHEN 'UserDeleteSelf' THEN 'UserManageSelf'
        WHEN 'UserProfile' THEN 'UserProfileSelf'
        WHEN 'DmCreate' THEN 'DmCreate'
        WHEN 'FriendCreate' THEN 'FriendCreate'
        WHEN 'RoomJoin' THEN 'RoomJoin'
        WHEN 'CallUpdate' THEN 'CallUpdate'
        WHEN 'RoomForceJoin' THEN 'RoomJoinForce'
        ELSE NULL
    END::permission;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Migrate role.allow and role.deny
ALTER TABLE role ADD COLUMN allow_new permission[];
ALTER TABLE role ADD COLUMN deny_new permission[];

UPDATE role SET allow_new = array(
    SELECT convert_permission(p::text)
    FROM unnest(allow) AS p
    WHERE p::text NOT IN ('ServerReports', 'TagApply', 'TagManage', 'ThreadLock', 'VoiceConnect')
    AND convert_permission(p::text) IS NOT NULL
);

UPDATE role SET deny_new = array(
    SELECT convert_permission(p::text)
    FROM unnest(deny) AS p
    WHERE p::text NOT IN ('ServerReports', 'TagApply', 'TagManage', 'ThreadLock', 'VoiceConnect')
    AND convert_permission(p::text) IS NOT NULL
);

ALTER TABLE role DROP COLUMN allow;
ALTER TABLE role RENAME COLUMN allow_new TO allow;
ALTER TABLE role ALTER COLUMN allow SET NOT NULL;

ALTER TABLE role DROP COLUMN deny;
ALTER TABLE role RENAME COLUMN deny_new TO deny;
ALTER TABLE role ALTER COLUMN deny SET NOT NULL;

-- Migrate permission_overwrite.allow and .deny
UPDATE permission_overwrite SET allow = (
    SELECT COALESCE(jsonb_agg(
        CASE elem::text
            WHEN '"MemberBridge"' THEN '"IntegrationsBridge"'
            WHEN '"ReactionPurge"' THEN '"ReactionManage"'
            WHEN '"RoomManage"' THEN '"RoomEdit"'
            WHEN '"RoomManageServer"' THEN '"RoomManage"'
            WHEN '"BypassSlowmode"' THEN '"ChannelSlowmodeBypass"'
            WHEN '"ViewChannel"' THEN '"ChannelView"'
            WHEN '"ViewAuditLog"' THEN '"AuditLogView"'
            WHEN '"ViewAnalytics"' THEN '"AnalyticsView"'
            WHEN '"VoiceDisconnect"' THEN '"VoiceMove"'
            WHEN '"UserDeleteSelf"' THEN '"UserManageSelf"'
            WHEN '"UserProfile"' THEN '"UserProfileSelf"'
            WHEN '"RoomForceJoin"' THEN '"RoomJoinForce"'
            ELSE elem
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(allow) AS elem
    WHERE elem::text NOT IN ('"ServerReports"', '"TagApply"', '"TagManage"', '"ThreadLock"', '"VoiceConnect"')
);

UPDATE permission_overwrite SET deny = (
    SELECT COALESCE(jsonb_agg(
        CASE elem::text
            WHEN '"MemberBridge"' THEN '"IntegrationsBridge"'
            WHEN '"ReactionPurge"' THEN '"ReactionManage"'
            WHEN '"RoomManage"' THEN '"RoomEdit"'
            WHEN '"RoomManageServer"' THEN '"RoomManage"'
            WHEN '"BypassSlowmode"' THEN '"ChannelSlowmodeBypass"'
            WHEN '"ViewChannel"' THEN '"ChannelView"'
            WHEN '"ViewAuditLog"' THEN '"AuditLogView"'
            WHEN '"ViewAnalytics"' THEN '"AnalyticsView"'
            WHEN '"VoiceDisconnect"' THEN '"VoiceMove"'
            WHEN '"UserDeleteSelf"' THEN '"UserManageSelf"'
            WHEN '"UserProfile"' THEN '"UserProfileSelf"'
            WHEN '"RoomForceJoin"' THEN '"RoomJoinForce"'
            ELSE elem
        END
    ), '[]'::jsonb)
    FROM jsonb_array_elements(deny) AS elem
    WHERE elem::text NOT IN ('"ServerReports"', '"TagApply"', '"TagManage"', '"ThreadLock"', '"VoiceConnect"')
);

DROP FUNCTION convert_permission(text);
DROP TYPE permission_old;
