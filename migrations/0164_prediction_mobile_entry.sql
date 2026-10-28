-- Keep the existing mobile entry visible until an administrator explicitly hides it.
ALTER TABLE prediction_settings
    ADD COLUMN mobile_entry_enabled BOOLEAN NULL DEFAULT TRUE;

UPDATE prediction_settings
SET mobile_entry_enabled = TRUE
WHERE mobile_entry_enabled IS NULL;

ALTER TABLE prediction_settings
    MODIFY COLUMN mobile_entry_enabled BOOLEAN NOT NULL DEFAULT TRUE;
