CREATE OR REPLACE FUNCTION content_hosts(content TEXT) RETURNS TEXT[] AS $$
BEGIN
    RETURN COALESCE(
        (
            SELECT array_agg(url_host(match[1]))
            FROM regexp_matches(content, '(https?://[^\s<>"]+|www\.[^\s<>"]+)', 'g') AS match
        ),
        ARRAY[]::TEXT[]
    );
END;
$$ LANGUAGE plpgsql IMMUTABLE;