CREATE TABLE role (
    id UUID PRIMARY KEY,
    room_id UUID,
    name TEXT NOT NULL,
    description TEXT,
    permissions TEXT[] NOT NULL,
    is_self_applicable BOOL NOT NULL,
    is_mentionable BOOL NOT NULL,
    is_default BOOL NOT NULL,
    FOREIGN KEY (room_id) REFERENCES room(id)
);

CREATE TABLE role_member (
    user_id UUID,
    role_id UUID,
    FOREIGN KEY (user_id) REFERENCES usr(id),
    FOREIGN KEY (role_id) REFERENCES role(id),
    PRIMARY KEY (user_id, role_id)
);
