CREATE TABLE IF NOT EXISTS discord_role_mapping (
    lamprey_role_id TEXT PRIMARY KEY,
    discord_role_id TEXT NOT NULL,
    discord_guild_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_discord_role_mapping_lookup
ON discord_role_mapping(discord_role_id, discord_guild_id);
