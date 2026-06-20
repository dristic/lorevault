-- Move credential storage out of `users` and into a provider-keyed table.
-- This lets us attach multiple auth providers to a single account later
-- (e.g. a user can log in with password OR GitHub OAuth).

CREATE TABLE user_identities (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Stable name for the provider: 'password', 'github', 'google', etc.
    provider        TEXT NOT NULL,
    -- Unique identifier within the provider's namespace.
    -- For 'password': lower(email).  For OAuth: the provider's user ID.
    provider_uid    TEXT NOT NULL,
    -- Provider-specific credential blob.
    -- 'password': { "hash": "<argon2 PHC string>" }
    -- OAuth:      null (identity is verified externally on every login)
    credential_json JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, provider_uid)
);

CREATE INDEX idx_user_identities_user ON user_identities (user_id);

-- Migrate any existing password hashes into the new table before dropping the column.
INSERT INTO user_identities (id, user_id, provider, provider_uid, credential_json)
SELECT
    gen_random_uuid(),
    id,
    'password',
    lower(email),
    jsonb_build_object('hash', password_hash)
FROM users
WHERE password_hash IS NOT NULL AND password_hash <> '';

ALTER TABLE users DROP COLUMN password_hash;
