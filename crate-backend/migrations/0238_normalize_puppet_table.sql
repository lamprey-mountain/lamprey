CREATE TABLE puppet (
    id UUID PRIMARY KEY REFERENCES usr(id) ON DELETE CASCADE,
    external_platform TEXT,
    external_id TEXT NOT NULL,
    external_url TEXT,
    alias_id UUID REFERENCES usr(id)
);

CREATE INDEX ON puppet (external_id);
CREATE INDEX ON usr (parent_id);

INSERT INTO puppet (id, external_platform, external_id, external_url, alias_id)
SELECT
    id,
    puppet->>'external_platform',
    puppet->>'external_id',
    puppet->>'external_url',
    (puppet->>'alias_id')::UUID
FROM usr
WHERE jsonb_typeof(puppet) = 'object' AND parent_id IS NOT NULL;

ALTER TABLE usr DROP COLUMN puppet;
