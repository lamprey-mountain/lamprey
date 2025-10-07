CREATE OR REPLACE FUNCTION embed_hosts(embeds JSONB) RETURNS TEXT[] AS $$
DECLARE
    hosts TEXT[];
    embed JSONB;
    url TEXT;
BEGIN
    IF embeds IS NULL THEN
        RETURN ARRAY[]::TEXT[];
    END IF;
    hosts := ARRAY[]::TEXT[];
    FOR embed IN SELECT * FROM jsonb_array_elements(embeds)
    LOOP
        url := embed->>'url';
        IF url IS NOT NULL THEN
            hosts := array_append(hosts, url_host(url));
        END IF;
    END LOOP;
    RETURN hosts;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE INDEX message_embed_hosts_gin_idx ON message USING GIN (embed_hosts(embeds));
CREATE INDEX message_pinned_idx ON message (id) WHERE pinned IS NOT NULL and is_latest and deleted_at is null;
