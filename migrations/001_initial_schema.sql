-- Custom types

CREATE TYPE owner_type AS ENUM ('user', 'org');
CREATE TYPE visibility AS ENUM ('public', 'private');
CREATE TYPE org_role AS ENUM ('owner', 'admin', 'member');
CREATE TYPE repo_role AS ENUM ('admin', 'write', 'read');

-- Users

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Organizations

CREATE TABLE organizations (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE org_members (
    org_id  UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role    org_role NOT NULL DEFAULT 'member',
    PRIMARY KEY (org_id, user_id)
);

-- Auth tokens & SSH keys

CREATE TABLE api_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    token_hash  TEXT NOT NULL UNIQUE,
    scopes      TEXT[] NOT NULL DEFAULT '{}',
    last_used   TIMESTAMPTZ,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ssh_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    public_key  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Repositories

CREATE TABLE repositories (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_type     owner_type NOT NULL,
    owner_id       UUID NOT NULL,
    name           TEXT NOT NULL,
    description    TEXT,
    visibility     visibility NOT NULL DEFAULT 'private',
    default_branch TEXT NOT NULL DEFAULT 'main',
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, name)
);

CREATE TABLE repo_permissions (
    repo_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role    repo_role NOT NULL DEFAULT 'read',
    PRIMARY KEY (repo_id, user_id)
);

-- Lore VCS objects

CREATE TABLE branches (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id              UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name                 TEXT NOT NULL,
    head_revision_hash   TEXT NOT NULL,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (repo_id, name)
);

CREATE TABLE revisions (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id        UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    hash           TEXT NOT NULL,
    parent_hashes  TEXT[] NOT NULL DEFAULT '{}',
    author_id      UUID NOT NULL REFERENCES users(id),
    message        TEXT NOT NULL DEFAULT '',
    timestamp      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (repo_id, hash)
);

-- CAS (content-addressed storage) index

CREATE TABLE chunks (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id      UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    hash         TEXT NOT NULL,
    size_bytes   BIGINT NOT NULL,
    storage_key  TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (repo_id, hash)
);

CREATE INDEX idx_chunks_hash ON chunks (hash);

-- File locks (Lore LockService)

CREATE TABLE file_locks (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repo_id            UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    path               TEXT NOT NULL,
    locked_by_user_id  UUID NOT NULL REFERENCES users(id),
    workspace_id       TEXT NOT NULL,
    acquired_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (repo_id, path)
);

CREATE INDEX idx_file_locks_repo ON file_locks (repo_id);
