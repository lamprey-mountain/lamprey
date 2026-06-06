ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'FriendRequestSent';
ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'FriendRequestReceived';
ALTER TYPE notification_type ADD VALUE IF NOT EXISTS 'FriendRequestAccepted';
ALTER TABLE inbox ADD COLUMN IF NOT EXISTS target_user_id UUID;
ALTER TABLE inbox ADD COLUMN IF NOT EXISTS reaction_key TEXT;
