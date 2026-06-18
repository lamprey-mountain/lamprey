create table portal (
    id integer primary key autoincrement,
    realm_id integer,
    
    -- Lamprey fields (nullable)
    lamprey_channel_id text,
    lamprey_room_id text,
    lamprey_last_id text,
    
    -- Discord fields (nullable)
    discord_guild_id text,
    discord_parent_id text,
    discord_channel_id text,
    discord_webhook_url text,
    discord_last_id text,
    
    foreign key (realm_id) references realm (id)
);
