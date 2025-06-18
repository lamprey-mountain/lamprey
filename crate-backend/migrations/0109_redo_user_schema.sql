-- Step 1: Rename old columns (optional, for backup/reference)
ALTER TABLE usr RENAME COLUMN type TO legacy_type;
ALTER TABLE usr RENAME COLUMN state TO legacy_state;
ALTER TABLE usr RENAME COLUMN state_updated_at TO legacy_state_updated_at;
ALTER TABLE usr RENAME COLUMN puppet_external_platform TO legacy_puppet_external_platform;
ALTER TABLE usr RENAME COLUMN puppet_external_id TO legacy_puppet_external_id;
ALTER TABLE usr RENAME COLUMN puppet_external_url TO legacy_puppet_external_url;
ALTER TABLE usr RENAME COLUMN puppet_alias_id TO legacy_puppet_alias_id;
ALTER TABLE usr RENAME COLUMN bot_is_bridge TO legacy_bot_is_bridge;
ALTER TABLE usr RENAME COLUMN bot_access TO legacy_bot_access;
ALTER TABLE usr RENAME COLUMN parent_id TO legacy_parent_id;

-- Step 2: Add new fields as per the new `User` schema
ALTER TABLE usr
  ADD COLUMN system BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN bot JSONB,
  ADD COLUMN puppet JSONB,
  ADD COLUMN guest JSONB,
  ADD COLUMN suspended JSONB,
  ADD COLUMN status TEXT NOT NULL DEFAULT 'offline',
  ADD COLUMN registered_at TIMESTAMP,
  ADD COLUMN deleted_at TIMESTAMP;

-- Step 3: (Optional) Copy legacy values into new structure
-- Example: Populate `bot` field from legacy columns
UPDATE usr
SET bot = jsonb_build_object(
  'owner', jsonb_build_object('user_id', legacy_parent_id::text),
  'access', legacy_bot_access::text,
  'is_bridge', legacy_bot_is_bridge
)
WHERE legacy_type = 'Bot';

UPDATE usr
SET puppet = jsonb_build_object(
  'owner_id', legacy_parent_id::text,
  'external_platform', legacy_puppet_external_platform,
  'external_id', legacy_puppet_external_id,
  'external_url', legacy_puppet_external_url,
  'alias_id', legacy_puppet_alias_id::text
)
WHERE legacy_type = 'Puppet';

UPDATE usr
SET system = true
WHERE legacy_type = 'System';

UPDATE usr
SET suspended = jsonb_build_object('at', legacy_state_updated_at)
WHERE legacy_state = 'Suspended';

UPDATE usr
SET deleted_at = legacy_state_updated_at
WHERE legacy_state = 'Deleted';

-- Step 4: (Optional) Drop legacy columns after migration verification
ALTER TABLE usr
  DROP COLUMN legacy_type,
  DROP COLUMN legacy_state,
  DROP COLUMN legacy_state_updated_at,
  DROP COLUMN legacy_puppet_external_platform,
  DROP COLUMN legacy_puppet_external_id,
  DROP COLUMN legacy_puppet_external_url,
  DROP COLUMN legacy_puppet_alias_id,
  DROP COLUMN legacy_bot_is_bridge,
  DROP COLUMN legacy_bot_access,
  DROP COLUMN legacy_parent_id;
