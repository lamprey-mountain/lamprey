---
-- Every room should have exactly one default role.
-- The default role has the same id as the room.
--

-- Function to merge two arrays, keeping unique elements.
CREATE OR REPLACE FUNCTION array_union(a anyarray, b anyarray)
RETURNS anyarray AS $$
SELECT ARRAY(SELECT unnest(a) UNION SELECT unnest(b));
$$ LANGUAGE SQL;

-- Create a temporary table to store merged permissions for default roles
CREATE TEMP TABLE room_default_permissions AS
SELECT room_id, array_agg(DISTINCT p) as permissions
FROM role, unnest(permissions) as p
WHERE is_default = true AND room_id IS NOT NULL
GROUP BY room_id;

-- Delete role_member entries for old default roles
DELETE FROM role_member WHERE role_id IN (SELECT id FROM role WHERE is_default = true);

-- Delete old default roles
DELETE FROM role WHERE is_default = true;

-- Drop the is_default column
ALTER TABLE role DROP COLUMN is_default;

-- Create a new default role for each room with the room's ID
-- On conflict (if a role with the same ID as a room already exists), update it to become the default role.
INSERT INTO role (id, room_id, name, description, permissions, is_self_applicable, is_mentionable, version_id)
SELECT
    r.id,
    r.id,
    'everyone',
    'Default role',
    COALESCE(rdp.permissions, ARRAY['MessageCreate', 'MessageAttachments', 'ReactionAdd', 'ThreadCreateChat', 'VoiceConnect', 'VoiceSpeak']::permission[]),
    false,
    true,
    r.id
FROM room r
LEFT JOIN room_default_permissions rdp ON r.id = rdp.room_id
ON CONFLICT (id) DO UPDATE SET
    name = 'everyone',
    description = 'Default role',
    permissions = array_union(role.permissions, EXCLUDED.permissions),
    is_self_applicable = false,
    is_mentionable = true;
