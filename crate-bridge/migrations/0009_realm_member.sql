CREATE TABLE realm_member (
    realm_id TEXT NOT NULL REFERENCES realm(id) ON DELETE CASCADE,
    lamprey_id TEXT NOT NULL REFERENCES "user"(lamprey_id) ON DELETE CASCADE,
    nickname TEXT,
    PRIMARY KEY (realm_id, lamprey_id)
);
