-- F026: promotions and options of the direct booking, offered by stay length.
-- Amounts in cents, percentages in basis points (1000 = 10 %). NULL bounds and dates are open.

CREATE TABLE promotions (
    id          BIGINT UNSIGNED   NOT NULL AUTO_INCREMENT,
    cottage_id  BIGINT UNSIGNED   NOT NULL,
    name_fr     VARCHAR(80)       NOT NULL,
    name_en     VARCHAR(80)       NOT NULL DEFAULT '',
    -- 'percent' (value in basis points of the nights), 'per_stay' or 'per_night' (cents).
    kind        VARCHAR(16)       NOT NULL,
    value       BIGINT UNSIGNED   NOT NULL,
    min_nights  SMALLINT UNSIGNED NULL,
    max_nights  SMALLINT UNSIGNED NULL,
    -- On the arrival date, both included.
    valid_from  DATE              NULL,
    valid_until DATE              NULL,
    active      BOOLEAN           NOT NULL DEFAULT TRUE,
    PRIMARY KEY (id),
    CONSTRAINT promotions_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT promotions_kind CHECK (kind IN ('percent', 'per_stay', 'per_night')),
    CONSTRAINT promotions_value CHECK (value > 0 AND (kind <> 'percent' OR value <= 10000)),
    CONSTRAINT promotions_bounds CHECK (min_nights IS NULL OR max_nights IS NULL OR min_nights <= max_nights),
    CONSTRAINT promotions_dates CHECK (valid_from IS NULL OR valid_until IS NULL OR valid_from <= valid_until)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE stay_options (
    id             BIGINT UNSIGNED   NOT NULL AUTO_INCREMENT,
    cottage_id     BIGINT UNSIGNED   NOT NULL,
    name_fr        VARCHAR(80)       NOT NULL,
    name_en        VARCHAR(80)       NOT NULL DEFAULT '',
    description_fr VARCHAR(300)      NOT NULL DEFAULT '',
    description_en VARCHAR(300)      NOT NULL DEFAULT '',
    -- 'stay', 'night', 'guest' or 'guest_night'; price 0: offered.
    unit           VARCHAR(16)       NOT NULL,
    price          BIGINT UNSIGNED   NOT NULL,
    min_nights     SMALLINT UNSIGNED NULL,
    max_nights     SMALLINT UNSIGNED NULL,
    position       SMALLINT UNSIGNED NOT NULL DEFAULT 0,
    active         BOOLEAN           NOT NULL DEFAULT TRUE,
    PRIMARY KEY (id),
    CONSTRAINT stay_options_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE,
    CONSTRAINT stay_options_unit CHECK (unit IN ('stay', 'night', 'guest', 'guest_night')),
    CONSTRAINT stay_options_bounds CHECK (min_nights IS NULL OR max_nights IS NULL OR min_nights <= max_nights)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
