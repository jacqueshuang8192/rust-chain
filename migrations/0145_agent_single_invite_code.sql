-- no-transaction
-- Keep one canonical agent code per agent before adding the database invariant.
-- The newest active code is canonical because it is the code previously exposed
-- by the user referral endpoint; when no active code exists, keep the newest row.
CREATE TEMPORARY TABLE agent_invite_code_canonical (
    agent_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    canonical_id BIGINT UNSIGNED NOT NULL,
    merged_used_count BIGINT UNSIGNED NOT NULL,
    merged_usage_limit BIGINT UNSIGNED NULL
);

INSERT INTO agent_invite_code_canonical (
    agent_id,
    canonical_id,
    merged_used_count,
    merged_usage_limit
)
SELECT
    ranked.owner_id,
    MAX(CASE WHEN ranked.rn = 1 THEN ranked.id END),
    LEAST(2147483647, SUM(CAST(ranked.used_count AS UNSIGNED))),
    CASE
        WHEN SUM(CASE WHEN ranked.usage_limit IS NULL THEN 1 ELSE 0 END) > 0 THEN NULL
        ELSE LEAST(2147483647, SUM(CAST(ranked.usage_limit AS UNSIGNED)))
    END
FROM (
    SELECT
        id,
        owner_id,
        usage_limit,
        used_count,
        ROW_NUMBER() OVER (
            PARTITION BY owner_id
            ORDER BY
                CASE WHEN status = 'active' THEN 0 ELSE 1 END,
                created_at DESC,
                id DESC
        ) AS rn
    FROM invite_codes
    WHERE owner_type = 'agent'
) ranked
GROUP BY ranked.owner_id;

UPDATE invite_codes codes
INNER JOIN agent_invite_code_canonical canonical
    ON canonical.canonical_id = codes.id
SET codes.used_count = canonical.merged_used_count,
    codes.usage_limit = canonical.merged_usage_limit;

DELETE codes
FROM invite_codes codes
INNER JOIN agent_invite_code_canonical canonical
    ON canonical.agent_id = codes.owner_id
WHERE codes.owner_type = 'agent'
  AND codes.id <> canonical.canonical_id;

ALTER TABLE invite_codes
    ADD COLUMN agent_owner_id BIGINT UNSIGNED
        GENERATED ALWAYS AS (
            CASE WHEN owner_type = 'agent' THEN owner_id ELSE NULL END
        ) STORED,
    ADD UNIQUE KEY uk_invite_codes_agent_owner (agent_owner_id);

DROP TEMPORARY TABLE agent_invite_code_canonical;
