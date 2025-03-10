-- unfortunately, the schema seems to have changed too much since practically
-- nobody touched has audit logs since they were implemented and its easier to
-- delete them than to migrate their schema, it's time for the first data loss
-- ever on cetahe!

truncate audit_log;
