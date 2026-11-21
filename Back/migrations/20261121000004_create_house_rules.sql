-- F004: house rules the quote depends on, edited by the owners ("Règles du gîte", ADR-0010).
-- Amounts in cents, rates in basis points (1000 = 10 %). Weekdays are a bit set, Monday = 1,
-- Tuesday = 2, … Sunday = 64 (127: every day).

ALTER TABLE booking_rules
    ADD COLUMN capacity                    TINYINT UNSIGNED  NOT NULL DEFAULT 5,
    ADD COLUMN min_delay_days              SMALLINT UNSIGNED NOT NULL DEFAULT 1,
    ADD COLUMN arrival_weekdays            TINYINT UNSIGNED  NOT NULL DEFAULT 127,
    ADD COLUMN departure_weekdays          TINYINT UNSIGNED  NOT NULL DEFAULT 127,
    ADD COLUMN deposit_mode                VARCHAR(16)       NOT NULL DEFAULT 'percentage',
    ADD COLUMN deposit_percent             TINYINT UNSIGNED  NOT NULL DEFAULT 50,
    ADD COLUMN deposit_fixed               BIGINT UNSIGNED   NOT NULL DEFAULT 0,
    ADD COLUMN balance_days                SMALLINT UNSIGNED NOT NULL DEFAULT 30,
    ADD COLUMN tourist_tax_per_adult_night BIGINT UNSIGNED   NOT NULL DEFAULT 0,
    ADD COLUMN tourist_tax_additional_bp   SMALLINT UNSIGNED NOT NULL DEFAULT 0,
    ADD COLUMN tourist_tax_exempt_under    TINYINT UNSIGNED  NOT NULL DEFAULT 18,
    ADD CONSTRAINT booking_rules_capacity_positive CHECK (capacity > 0),
    ADD CONSTRAINT booking_rules_weekdays CHECK (arrival_weekdays BETWEEN 1 AND 127
                                                 AND departure_weekdays BETWEEN 1 AND 127),
    ADD CONSTRAINT booking_rules_deposit CHECK (deposit_mode IN ('percentage', 'fixed')
                                                AND deposit_percent BETWEEN 1 AND 100),
    ADD CONSTRAINT booking_rules_additional_tax CHECK (tourist_tax_additional_bp <= 10000);

-- Every change of the house rules, with its author (kept when the account is deleted).
CREATE TABLE house_rule_changes (
    id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    cottage_id   BIGINT UNSIGNED NOT NULL,
    description  VARCHAR(500)    NOT NULL,
    before_value JSON            NULL,
    after_value  JSON            NULL,
    author_id    BIGINT UNSIGNED NULL,
    at           DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY house_rule_changes_cottage (cottage_id, at),
    CONSTRAINT house_rule_changes_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT house_rule_changes_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
