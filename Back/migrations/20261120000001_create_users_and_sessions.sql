-- F001 / ADR-0011: admin accounts and server-side sessions.
-- Timestamps are stored in UTC.

CREATE TABLE users (
    id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    email         VARCHAR(254)    NOT NULL,
    password_hash VARCHAR(255)    NOT NULL,
    created_at    DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT users_email_unique UNIQUE (email)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE sessions (
    -- Hex SHA-256 of the session token; the token itself is never stored.
    token_hash   CHAR(64)        NOT NULL,
    user_id      BIGINT UNSIGNED NOT NULL,
    created_at   DATETIME(3)     NOT NULL,
    last_used_at DATETIME(3)     NOT NULL,
    expires_at   DATETIME(3)     NOT NULL,
    PRIMARY KEY (token_hash),
    KEY sessions_user_id (user_id),
    KEY sessions_expires_at (expires_at),
    CONSTRAINT sessions_user_fk FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
