-- F003: media library (ADR-0017). Files live on disk, named after the content hash;
-- this table keeps their dimensions, published widths and descriptive texts.

CREATE TABLE media (
    id                 BIGINT UNSIGNED  NOT NULL AUTO_INCREMENT,
    content_hash       CHAR(64)         NOT NULL,
    original_extension VARCHAR(8)       NOT NULL,
    width              INT UNSIGNED     NOT NULL,
    height             INT UNSIGNED     NOT NULL,
    -- Widths of the WebP versions, ascending (JSON array of integers).
    widths             JSON             NOT NULL,
    alt_fr             VARCHAR(300)     NOT NULL DEFAULT '',
    alt_en             VARCHAR(300)     NOT NULL DEFAULT '',
    caption_fr         VARCHAR(300)     NOT NULL DEFAULT '',
    caption_en         VARCHAR(300)     NOT NULL DEFAULT '',
    focal_x            TINYINT UNSIGNED NOT NULL DEFAULT 50,
    focal_y            TINYINT UNSIGNED NOT NULL DEFAULT 50,
    created_at         DATETIME(3)      NOT NULL,
    updated_at         DATETIME(3)      NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT media_content_hash_unique UNIQUE (content_hash),
    CONSTRAINT media_focal_x_range CHECK (focal_x <= 100),
    CONSTRAINT media_focal_y_range CHECK (focal_y <= 100)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
