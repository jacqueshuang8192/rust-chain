ALTER TABLE registration_legal_documents ADD COLUMN body_format VARCHAR(16) NULL;

UPDATE registration_legal_documents SET body_format = 'plain' WHERE body_format IS NULL;

ALTER TABLE registration_legal_documents MODIFY COLUMN body_format VARCHAR(16) NOT NULL DEFAULT 'plain';

ALTER TABLE registration_legal_documents ADD CONSTRAINT chk_registration_legal_body_format CHECK (body_format IN ('plain', 'rich_json'));
