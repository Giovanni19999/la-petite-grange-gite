-- F025 (ADR-0019): settings of the email and SMS sending, and the sending queue.
-- Secrets (SMTP password, Brevo API key) are never stored here: they come from the environment.

CREATE TABLE messaging_settings (
    id                  TINYINT UNSIGNED  NOT NULL,
    -- Empty host: emails are not configured yet.
    smtp_host           VARCHAR(255)      NOT NULL DEFAULT '',
    smtp_port           SMALLINT UNSIGNED NOT NULL DEFAULT 587,
    -- 'starttls', 'tls' or 'none' (local mail catcher only).
    smtp_security       VARCHAR(16)       NOT NULL DEFAULT 'starttls',
    smtp_username       VARCHAR(255)      NOT NULL DEFAULT '',
    from_address        VARCHAR(254)      NOT NULL DEFAULT '',
    from_name           VARCHAR(80)       NOT NULL DEFAULT '',
    reply_to            VARCHAR(254)      NULL,
    sms_enabled         BOOLEAN           NOT NULL DEFAULT FALSE,
    sms_sender          VARCHAR(11)       NOT NULL DEFAULT 'PetitGrange',
    -- JSON arrays of strings: notification recipients of the owners.
    owner_emails        TEXT              NOT NULL DEFAULT '[]',
    owner_phones        TEXT              NOT NULL DEFAULT '[]',
    max_attempts        TINYINT UNSIGNED  NOT NULL DEFAULT 5,
    first_retry_minutes SMALLINT UNSIGNED NOT NULL DEFAULT 5,
    PRIMARY KEY (id),
    CONSTRAINT messaging_settings_single CHECK (id = 1),
    CONSTRAINT messaging_settings_security CHECK (smtp_security IN ('starttls', 'tls', 'none')),
    CONSTRAINT messaging_settings_emails CHECK (JSON_VALID(owner_emails)),
    CONSTRAINT messaging_settings_phones CHECK (JSON_VALID(owner_phones)),
    CONSTRAINT messaging_settings_attempts CHECK (max_attempts BETWEEN 1 AND 20),
    CONSTRAINT messaging_settings_retry CHECK (first_retry_minutes BETWEEN 1 AND 1440)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

INSERT INTO messaging_settings (id) VALUES (1);

CREATE TABLE outgoing_messages (
    id              BIGINT UNSIGNED  NOT NULL AUTO_INCREMENT,
    -- Unique key of the message (e.g. 'booking:42:confirmation:guest:email'): never queued twice.
    message_key     VARCHAR(190)     NOT NULL,
    -- 'email' or 'sms'.
    channel         VARCHAR(8)       NOT NULL,
    recipient       VARCHAR(254)     NOT NULL,
    subject         VARCHAR(255)     NOT NULL DEFAULT '',
    body_text       TEXT             NOT NULL,
    body_html       MEDIUMTEXT       NULL,
    -- 'pending', 'sending', 'sent', 'failed', 'to_check' (interrupted) or 'disabled' (SMS off).
    status          VARCHAR(16)      NOT NULL DEFAULT 'pending',
    attempts        TINYINT UNSIGNED NOT NULL DEFAULT 0,
    next_attempt_at DATETIME         NOT NULL,
    -- Kind of the last error (never the body nor a secret).
    last_error      VARCHAR(32)      NULL,
    created_at      DATETIME         NOT NULL,
    sent_at         DATETIME         NULL,
    PRIMARY KEY (id),
    UNIQUE KEY outgoing_messages_key (message_key),
    KEY outgoing_messages_due (status, next_attempt_at),
    CONSTRAINT outgoing_messages_channel CHECK (channel IN ('email', 'sms')),
    CONSTRAINT outgoing_messages_status
        CHECK (status IN ('pending', 'sending', 'sent', 'failed', 'to_check', 'disabled'))
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;
