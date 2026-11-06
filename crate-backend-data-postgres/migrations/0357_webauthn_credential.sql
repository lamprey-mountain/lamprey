CREATE TABLE webauthn_credential (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    credential_id BYTEA NOT NULL UNIQUE,
    passkey TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    last_used_at TIMESTAMP
);

CREATE INDEX webauthn_credential_user_id_idx ON webauthn_credential (user_id);
CREATE INDEX webauthn_credential_credential_id_idx ON webauthn_credential (credential_id);
