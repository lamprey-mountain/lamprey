ALTER TABLE oauth_authorization_code ADD COLUMN code_challenge TEXT;
ALTER TABLE oauth_authorization_code ADD COLUMN code_challenge_method TEXT;
