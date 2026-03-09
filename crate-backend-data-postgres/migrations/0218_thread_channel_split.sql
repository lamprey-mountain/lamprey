-- rename everything
ALTER TABLE thread RENAME TO channel;
ALTER INDEX thread_pkey RENAME TO channel_pkey;
ALTER TABLE channel RENAME CONSTRAINT thread_room_id_fkey TO channel_room_id_fkey;
ALTER TABLE channel RENAME CONSTRAINT thread_creator_id_fkey TO channel_creator_id_fkey;
ALTER TABLE channel RENAME CONSTRAINT fk_thread_owner TO fk_channel_owner;
ALTER TABLE channel RENAME CONSTRAINT thread_parent_id_fkey TO channel_parent_id_fkey;
ALTER TABLE channel RENAME CONSTRAINT thread_icon_fkey TO channel_icon_fkey;
ALTER TYPE thread_type RENAME TO channel_type;
ALTER TABLE thread_member RENAME TO channel_member;
ALTER TABLE channel_member RENAME COLUMN thread_id TO channel_id;
ALTER TABLE channel_member RENAME CONSTRAINT thread_member_pkey TO channel_member_pkey;
ALTER TABLE channel_member RENAME CONSTRAINT thread_member_thread_id_fkey TO channel_member_channel_id_fkey;
ALTER TABLE channel_member RENAME CONSTRAINT thread_member_user_id_fkey TO channel_member_user_id_fkey;
ALTER TABLE message RENAME COLUMN thread_id TO channel_id;
ALTER TABLE message RENAME CONSTRAINT message_thread_id_fkey TO message_channel_id_fkey;
ALTER INDEX idx_message_thread_latest RENAME TO idx_message_channel_latest;
ALTER INDEX idx_message_latest_filtered RENAME TO idx_message_channel_filtered;
ALTER TABLE unread RENAME COLUMN thread_id TO channel_id;
ALTER TABLE unread RENAME CONSTRAINT unread_pkey TO unread_channel_pkey;
ALTER TABLE unread RENAME CONSTRAINT unread_thread_id_fkey TO unread_channel_id_fkey;
ALTER TABLE inbox RENAME COLUMN thread_id TO channel_id;
ALTER TABLE inbox RENAME CONSTRAINT inbox_thread_id_fkey TO inbox_channel_id_fkey;
ALTER TABLE dm RENAME COLUMN thread_id TO channel_id;
ALTER TABLE dm RENAME CONSTRAINT dm_thread_id_fkey TO dm_channel_id_fkey;
ALTER TABLE room RENAME COLUMN welcome_thread_id TO welcome_channel_id;
ALTER TABLE room RENAME CONSTRAINT room_welcome_thread_id_fkey TO room_welcome_channel_id_fkey;
ALTER TABLE user_config_thread RENAME TO user_config_channel;
ALTER TABLE user_config_channel RENAME COLUMN thread_id TO channel_id;
ALTER TABLE user_config_channel RENAME CONSTRAINT user_config_thread_pkey TO user_config_channel_pkey;
ALTER TABLE user_config_channel RENAME CONSTRAINT user_config_thread_thread_id_fkey TO user_config_channel_channel_id_fkey;
ALTER TABLE user_config_channel RENAME CONSTRAINT user_config_thread_user_id_fkey TO user_config_channel_user_id_fkey;

-- removed unused stuff
drop table tag_apply_room;
drop table tag_apply_thread;
drop table tag;

-- update permissions
ALTER TYPE permission RENAME VALUE 'ViewThread' TO 'ViewChannel';
ALTER TYPE permission ADD VALUE 'ChannelManage';
ALTER TYPE permission ADD VALUE 'ChannelEdit';
