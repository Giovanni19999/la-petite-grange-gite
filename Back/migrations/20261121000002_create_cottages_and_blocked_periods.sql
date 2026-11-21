-- F005: the cottage, its booking horizon, and the periods its owners close.
-- Dates are calendar dates of the cottage (Europe/Paris); a period's end is excluded,
-- like a stay. Timestamps are UTC.

CREATE TABLE cottages (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    slug       VARCHAR(80)     NOT NULL,
    name       VARCHAR(120)    NOT NULL,
    created_at DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT cottages_slug_unique UNIQUE (slug)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- House rules of each cottage (ADR-0010). F005 only needs the booking horizon; the other
-- rules arrive with the booking features (F007).
CREATE TABLE booking_rules (
    cottage_id        BIGINT UNSIGNED  NOT NULL,
    reservable_months SMALLINT UNSIGNED NOT NULL,
    reservable_until  DATE             NULL,
    updated_at        DATETIME(3)      NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (cottage_id),
    CONSTRAINT booking_rules_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT booking_rules_months_positive CHECK (reservable_months > 0)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE blocked_periods (
    id         BIGINT UNSIGNED  NOT NULL AUTO_INCREMENT,
    cottage_id BIGINT UNSIGNED  NOT NULL,
    start_date DATE             NOT NULL,
    -- NULL: closed until further notice.
    end_date   DATE             NULL,
    kind       VARCHAR(20)      NOT NULL,
    label      VARCHAR(80)      NULL,
    people     TINYINT UNSIGNED NULL,
    reason     VARCHAR(300)     NULL,
    created_at DATETIME(3)      NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at DATETIME(3)      NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    KEY blocked_periods_dates (cottage_id, start_date, end_date),
    CONSTRAINT blocked_periods_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT blocked_periods_kind CHECK (kind IN ('closure', 'family', 'private_event')),
    CONSTRAINT blocked_periods_order CHECK (end_date IS NULL OR end_date > start_date)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- History of the changes (kept after a period is deleted).
CREATE TABLE blocked_period_events (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    period_id  BIGINT UNSIGNED NOT NULL,
    cottage_id BIGINT UNSIGNED NOT NULL,
    action     VARCHAR(10)     NOT NULL,
    before_value JSON          NULL,
    after_value  JSON          NULL,
    author_id  BIGINT UNSIGNED NULL,
    at         DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY blocked_period_events_cottage (cottage_id, at),
    CONSTRAINT blocked_period_events_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT blocked_period_events_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL,
    CONSTRAINT blocked_period_events_action CHECK (action IN ('created', 'updated', 'deleted'))
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Seed (ADR-0010): the cottage and its default horizon, editable later from the admin.
INSERT INTO cottages (slug, name) VALUES ('la-petite-grange', 'La Petite Grange');
INSERT INTO booking_rules (cottage_id, reservable_months)
    SELECT id, 24 FROM cottages WHERE slug = 'la-petite-grange';
