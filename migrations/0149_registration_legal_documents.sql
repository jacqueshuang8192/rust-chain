CREATE TABLE registration_legal_documents (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scope_code VARCHAR(8) NOT NULL,
    locale VARCHAR(16) NOT NULL,
    kind VARCHAR(16) NOT NULL,
    title VARCHAR(160) NOT NULL,
    body_text LONGTEXT NOT NULL,
    effective_on DATE NOT NULL,
    published_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    published_by BIGINT UNSIGNED NOT NULL,
    CONSTRAINT chk_registration_legal_kind CHECK (kind IN ('terms', 'privacy')),
    CONSTRAINT chk_registration_legal_scope CHECK (CHAR_LENGTH(TRIM(scope_code)) > 0),
    CONSTRAINT chk_registration_legal_body CHECK (CHAR_LENGTH(TRIM(body_text)) > 0),
    INDEX idx_registration_legal_scope (scope_code, locale, kind, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE registration_legal_publications (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    scope_code VARCHAR(8) NOT NULL,
    locale VARCHAR(16) NOT NULL,
    edition BIGINT UNSIGNED NOT NULL,
    terms_id BIGINT UNSIGNED NOT NULL,
    privacy_id BIGINT UNSIGNED NOT NULL,
    published_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    published_by BIGINT UNSIGNED NOT NULL,
    UNIQUE KEY uq_registration_legal_edition (scope_code, locale, edition),
    CONSTRAINT fk_registration_legal_terms FOREIGN KEY (terms_id) REFERENCES registration_legal_documents(id),
    CONSTRAINT fk_registration_legal_privacy FOREIGN KEY (privacy_id) REFERENCES registration_legal_documents(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE registration_legal_current (
    scope_code VARCHAR(8) NOT NULL,
    locale VARCHAR(16) NOT NULL,
    edition BIGINT UNSIGNED NOT NULL DEFAULT 0,
    publication_id BIGINT UNSIGNED NULL,
    PRIMARY KEY (scope_code, locale),
    CONSTRAINT fk_registration_legal_current FOREIGN KEY (publication_id) REFERENCES registration_legal_publications(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE user_registration_legal_receipts (
    user_id BIGINT UNSIGNED NOT NULL PRIMARY KEY,
    country_code VARCHAR(8) NOT NULL,
    scope_code VARCHAR(8) NOT NULL,
    locale VARCHAR(16) NOT NULL,
    publication_id BIGINT UNSIGNED NOT NULL,
    terms_id BIGINT UNSIGNED NOT NULL,
    privacy_id BIGINT UNSIGNED NOT NULL,
    acknowledged_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_user_registration_legal_user FOREIGN KEY (user_id) REFERENCES users(id),
    CONSTRAINT fk_user_registration_legal_publication FOREIGN KEY (publication_id) REFERENCES registration_legal_publications(id),
    CONSTRAINT fk_user_registration_legal_terms FOREIGN KEY (terms_id) REFERENCES registration_legal_documents(id),
    CONSTRAINT fk_user_registration_legal_privacy FOREIGN KEY (privacy_id) REFERENCES registration_legal_documents(id),
    INDEX idx_user_registration_legal_publication (publication_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
