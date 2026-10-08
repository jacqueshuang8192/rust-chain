CREATE TABLE home_carousel_configs (
    id TINYINT UNSIGNED PRIMARY KEY,
    revision BIGINT UNSIGNED NOT NULL DEFAULT 0,
    slides JSON NOT NULL,
    updated_by_admin_id BIGINT UNSIGNED NULL,
    updated_at TIMESTAMP(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6) ON UPDATE CURRENT_TIMESTAMP(6),
    CONSTRAINT fk_home_carousel_configs_admin
        FOREIGN KEY (updated_by_admin_id) REFERENCES admin_users(id),
    CONSTRAINT chk_home_carousel_configs_id CHECK (id = 1),
    CONSTRAINT chk_home_carousel_configs_slides CHECK (JSON_VALID(slides))
) COMMENT='首页推广轮播单例配置';

INSERT INTO home_carousel_configs (id, revision, slides)
VALUES (1, 0, JSON_OBJECT('slides', JSON_ARRAY()));
