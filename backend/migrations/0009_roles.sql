CREATE TABLE IF NOT EXISTS roles (
    id UUID PRIMARY KEY,
    room_id UUID,
    name TEXT NOT NULL,
    description TEXT,
    permissions TEXT[] NOT NULL,
    FOREIGN KEY (room_id) REFERENCES rooms(id)
);

CREATE TABLE IF NOT EXISTS roles_members (
    user_id UUID,
    role_id UUID,
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (role_id) REFERENCES roles(id),
    PRIMARY KEY (user_id, role_id)
);
