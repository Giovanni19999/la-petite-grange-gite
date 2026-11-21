-- F006: prices per date range and per channel, typed by hand (ADR-0008).
-- Amounts in cents; percentages in basis points (1550 = 15.50 %); dates are calendar dates
-- of the cottage, a range's end excluded.

ALTER TABLE cottages
    ADD COLUMN cleaning_fee       BIGINT UNSIGNED  NOT NULL DEFAULT 0,
    ADD COLUMN default_min_nights TINYINT UNSIGNED NOT NULL DEFAULT 2;

ALTER TABLE booking_rules
    -- Warning when the implicit commission differs more than this from the channel's rate.
    ADD COLUMN commission_alert_points TINYINT UNSIGNED NOT NULL DEFAULT 5;

CREATE TABLE channels (
    id                BIGINT UNSIGNED   NOT NULL AUTO_INCREMENT,
    code              VARCHAR(20)       NOT NULL,
    name              VARCHAR(60)       NOT NULL,
    commission_bp     SMALLINT UNSIGNED NOT NULL DEFAULT 0,
    payment_fees_bp   SMALLINT UNSIGNED NOT NULL DEFAULT 0,
    vat_on_commission BOOLEAN           NOT NULL DEFAULT FALSE,
    position          TINYINT UNSIGNED  NOT NULL DEFAULT 0,
    PRIMARY KEY (id),
    CONSTRAINT channels_code_unique UNIQUE (code),
    CONSTRAINT channels_rates CHECK (commission_bp <= 10000 AND payment_fees_bp <= 10000)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE rate_periods (
    id         BIGINT UNSIGNED  NOT NULL AUTO_INCREMENT,
    cottage_id BIGINT UNSIGNED  NOT NULL,
    start_date DATE             NOT NULL,
    end_date   DATE             NOT NULL,
    label      VARCHAR(60)      NULL,
    min_nights TINYINT UNSIGNED NOT NULL,
    created_at DATETIME(3)      NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    KEY rate_periods_dates (cottage_id, start_date, end_date),
    CONSTRAINT rate_periods_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT rate_periods_order CHECK (end_date > start_date),
    CONSTRAINT rate_periods_min_nights CHECK (min_nights BETWEEN 1 AND 30)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Prices of a channel: on a range, or the channel's default price (rate_period_id NULL).
CREATE TABLE channel_prices (
    id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    cottage_id     BIGINT UNSIGNED NOT NULL,
    rate_period_id BIGINT UNSIGNED NULL,
    channel_id     BIGINT UNSIGNED NOT NULL,
    listed         BIGINT UNSIGNED NOT NULL,
    net            BIGINT UNSIGNED NOT NULL,
    -- One price per (range or default, channel): NULL ranges compare equal through this key.
    period_key     BIGINT UNSIGNED AS (IFNULL(rate_period_id, 0)) PERSISTENT,
    PRIMARY KEY (id),
    CONSTRAINT channel_prices_unique UNIQUE (cottage_id, period_key, channel_id),
    CONSTRAINT channel_prices_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT channel_prices_period_fk FOREIGN KEY (rate_period_id) REFERENCES rate_periods (id) ON DELETE CASCADE,
    CONSTRAINT channel_prices_channel_fk FOREIGN KEY (channel_id) REFERENCES channels (id) ON DELETE CASCADE,
    CONSTRAINT channel_prices_amounts CHECK (listed > 0 AND net <= listed)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE rate_changes (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    cottage_id   BIGINT UNSIGNED NOT NULL,
    description  VARCHAR(300)    NOT NULL,
    before_value JSON            NULL,
    after_value  JSON            NULL,
    author_id    BIGINT UNSIGNED NULL,
    at           DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY rate_changes_cottage (cottage_id, at),
    CONSTRAINT rate_changes_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT rate_changes_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Seed (ADR-0010, plan § 9, to confirm on invoices): every value is editable in the admin.
INSERT INTO channels (code, name, commission_bp, payment_fees_bp, vat_on_commission, position) VALUES
    ('direct', 'Site', 0, 0, FALSE, 0),
    ('airbnb', 'Airbnb', 1550, 0, TRUE, 1),
    ('booking', 'Booking', 1500, 140, TRUE, 2);

INSERT INTO channel_prices (cottage_id, rate_period_id, channel_id, listed, net)
    SELECT c.id, NULL, ch.id, 10000, 10000
    FROM cottages c JOIN channels ch ON ch.code = 'direct'
    WHERE c.slug = 'la-petite-grange';
