-- Keep application logos immutable until approval; upload ownership is actor-specific.
ALTER TABLE agent_new_coin_applications
    ADD COLUMN logo_url VARCHAR(2048) NULL AFTER name;
ALTER TABLE upload_objects
    ADD COLUMN uploaded_by_agent BIGINT UNSIGNED NULL AFTER uploaded_by_user,
    ADD INDEX idx_upload_objects_uploaded_by_agent (uploaded_by_agent),
    ADD CONSTRAINT fk_upload_objects_uploaded_by_agent FOREIGN KEY (uploaded_by_agent) REFERENCES agent_admin_users(id);
