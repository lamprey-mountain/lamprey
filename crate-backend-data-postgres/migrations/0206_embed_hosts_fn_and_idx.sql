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
