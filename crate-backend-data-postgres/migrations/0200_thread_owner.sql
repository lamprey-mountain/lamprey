ALTER TABLE thread ADD COLUMN owner_id UUID;
UPDATE thread SET owner_id = creator_id;
ALTER TABLE thread ALTER COLUMN owner_id SET NOT NULL;
ALTER TABLE thread ADD CONSTRAINT fk_thread_owner FOREIGN KEY (owner_id) REFERENCES usr(id);
