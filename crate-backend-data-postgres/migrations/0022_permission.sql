create type permission as enum (
    'Admin',
    'RoomManage',
    'ThreadCreate',
    'ThreadManage',
    'ThreadDelete',
    'MessageCreate',
    'MessageFilesEmbeds',
    'MessagePin',
    'MessageDelete',
    'MessageMassMention',
    'MemberKick',
    'MemberBan',
    'MemberManage',
    'InviteCreate',
    'InviteManage',
    'RoleManage',
    'RoleApply',

    'View',
    'MessageEdit'
);

alter table role alter column permissions type permission[] using permissions::permission[];
