delete from media_link
where link_type in ('UserAvatar', 'UserBanner')
and target_id in (select id from usr where deleted_at is not null);

delete from usr
where deleted_at is not null;
