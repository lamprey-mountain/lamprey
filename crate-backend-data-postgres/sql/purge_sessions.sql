delete from session where expires_at < now();
