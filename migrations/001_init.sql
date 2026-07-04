-- Users

CREATE TABLE users (
    id         TEXT PRIMARY KEY,
    username   TEXT NOT NULL UNIQUE,
    email      TEXT NOT NULL UNIQUE,
    is_admin   INTEGER NOT NULL DEFAULT 0 CHECK (is_admin IN (0, 1)),
    must_change_password INTEGER NOT NULL DEFAULT 0 CHECK (must_change_password IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Provider-keyed credentials (password, OAuth, etc.)

CREATE TABLE user_identities (
    id              TEXT PRIMARY KEY,
    user_id         TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider        TEXT NOT NULL,
    provider_uid    TEXT NOT NULL,
    credential_json TEXT,
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (provider, provider_uid)
);

CREATE INDEX idx_user_identities_user ON user_identities (user_id);

-- Auth tokens & SSH keys

CREATE TABLE api_tokens (
    id         TEXT PRIMARY KEY,
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    scopes     TEXT NOT NULL DEFAULT '[]',
    last_used  TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);

-- Repositories (metadata only — VCS data lives on the external lore-server)

CREATE TABLE repositories (
    id             TEXT PRIMARY KEY,
    owner_id       TEXT NOT NULL,
    name           TEXT NOT NULL,
    description    TEXT,
    visibility     TEXT NOT NULL DEFAULT 'private' CHECK (visibility IN ('public', 'private')),
    default_branch TEXT NOT NULL DEFAULT 'main',
    created_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    UNIQUE (owner_id, name)
);

CREATE TABLE repo_permissions (
    repo_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role    TEXT NOT NULL DEFAULT 'read' CHECK (role IN ('admin', 'write', 'read')),
    PRIMARY KEY (repo_id, user_id)
);

-- Browser-based auth sessions (Lore CLI device flow — replaces Redis)

CREATE TABLE auth_sessions (
    code       TEXT PRIMARY KEY,
    state      TEXT NOT NULL DEFAULT 'pending',
    token      TEXT,
    user_id    TEXT,
    username   TEXT,
    expires_at INTEGER NOT NULL
);
