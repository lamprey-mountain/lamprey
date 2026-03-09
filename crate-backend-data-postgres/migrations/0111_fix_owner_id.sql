UPDATE usr 
SET bot = bot - 'owner_type' - 'owner' || jsonb_build_object('owner_id', bot->'owner'->>'user_id')
WHERE bot ? 'owner';
