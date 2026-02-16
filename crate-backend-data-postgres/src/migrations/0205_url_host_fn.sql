CREATE OR REPLACE FUNCTION url_host(url TEXT) RETURNS TEXT AS $$
BEGIN
    RETURN (regexp_matches(url, '^((?:https?://)?(?:[^@\n]+@)?(?:www\.)?([^:/\n?]+))'))[1];
EXCEPTION WHEN others THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

