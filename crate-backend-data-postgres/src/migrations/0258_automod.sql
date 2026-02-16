CREATE TABLE automod_rule (
    id UUID PRIMARY KEY,
    room_id UUID NOT NULL REFERENCES room(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    enabled BOOLEAN NOT NULL,
    data JSONB NOT NULL
);

CREATE TABLE automod_rule_except_role (
    rule_id UUID NOT NULL REFERENCES automod_rule(id) ON DELETE CASCADE,
    room_id UUID NOT NULL,
    role_id UUID NOT NULL REFERENCES role(id) ON DELETE CASCADE,
    PRIMARY KEY (rule_id, role_id)
);

CREATE INDEX automod_rule_except_role_room_id_idx ON automod_rule_except_role (room_id);

CREATE TABLE automod_rule_except_channel (
    rule_id UUID NOT NULL REFERENCES automod_rule(id) ON DELETE CASCADE,
    room_id UUID NOT NULL,
    channel_id UUID NOT NULL REFERENCES channel(id) ON DELETE CASCADE,
    PRIMARY KEY (rule_id, channel_id)
);

CREATE INDEX automod_rule_except_channel_room_id_idx ON automod_rule_except_channel (room_id);
