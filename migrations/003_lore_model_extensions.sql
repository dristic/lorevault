-- Add Lore-protocol fields to repositories and branches

ALTER TABLE repositories
    ADD COLUMN IF NOT EXISTS default_branch_uuid UUID,
    ADD COLUMN IF NOT EXISTS repo_metadata       BYTEA;

ALTER TABLE branches
    ADD COLUMN IF NOT EXISTS creator    TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS category   TEXT NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS deleted    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS metadata   BYTEA;

-- Add revision sequence numbers
ALTER TABLE revisions
    ADD COLUMN IF NOT EXISTS number BIGINT;

-- Backfill: assign sequential numbers within each repo ordered by timestamp
WITH numbered AS (
    SELECT id,
           ROW_NUMBER() OVER (PARTITION BY repo_id ORDER BY timestamp) AS rn
    FROM revisions
)
UPDATE revisions r
SET number = n.rn
FROM numbered n
WHERE r.id = n.id AND r.number IS NULL;
