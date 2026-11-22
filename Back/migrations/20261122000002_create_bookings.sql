-- F007: direct bookings without online payment (ADR-0020), payment instructions, editable
-- message templates (ADR-0021). Amounts in cents, times of day in minutes after midnight,
-- timestamps UTC, dates are calendar dates of the cottage (a stay's departure is excluded).

ALTER TABLE booking_rules
    ADD COLUMN arrival_from       SMALLINT UNSIGNED NOT NULL DEFAULT 960,
    ADD COLUMN departure_before   SMALLINT UNSIGNED NOT NULL DEFAULT 600,
    ADD COLUMN pets_allowed       BOOLEAN           NOT NULL DEFAULT FALSE,
    ADD COLUMN deposit_alert_days SMALLINT UNSIGNED NOT NULL DEFAULT 7,
    ADD COLUMN terms_page_id      BIGINT UNSIGNED   NULL,
    ADD CONSTRAINT booking_rules_terms_page_fk FOREIGN KEY (terms_page_id) REFERENCES pages (id) ON DELETE SET NULL,
    ADD CONSTRAINT booking_rules_times CHECK (arrival_from < 1440 AND departure_before < 1440),
    ADD CONSTRAINT booking_rules_alert_days CHECK (deposit_alert_days > 0);

-- How to pay offline, shown to the guests; the bank details document is a file of the private
-- directory of the API (never served by Caddy).
CREATE TABLE payment_settings (
    cottage_id      BIGINT UNSIGNED NOT NULL,
    instructions_fr MEDIUMTEXT      NOT NULL,
    instructions_en MEDIUMTEXT      NOT NULL,
    methods         JSON            NOT NULL,
    document_name   VARCHAR(200)    NULL,
    document_type   VARCHAR(100)    NULL,
    document_file   VARCHAR(100)    NULL,
    updated_at      DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (cottage_id),
    CONSTRAINT payment_settings_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id) ON DELETE CASCADE
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

INSERT INTO payment_settings (cottage_id, instructions_fr, instructions_en, methods)
SELECT id, '', '', '["Virement", "Chèque", "Chèques-vacances", "Espèces"]' FROM cottages;

CREATE TABLE bookings (
    id                    BIGINT UNSIGNED   NOT NULL AUTO_INCREMENT,
    cottage_id            BIGINT UNSIGNED   NOT NULL,
    reference             VARCHAR(20)       NOT NULL,
    reference_year        SMALLINT UNSIGNED NOT NULL,
    reference_number      INT UNSIGNED      NOT NULL,
    -- Random tracking token of the guest's link (never the id).
    token                 VARCHAR(64)       NOT NULL,
    status                VARCHAR(20)       NOT NULL,
    payment_mode          VARCHAR(10)       NOT NULL,
    lang                  CHAR(2)           NOT NULL,
    arrival               DATE              NOT NULL,
    departure             DATE              NOT NULL,
    adults                TINYINT UNSIGNED  NOT NULL,
    children              TINYINT UNSIGNED  NOT NULL,
    first_name            VARCHAR(100)      NOT NULL,
    last_name             VARCHAR(100)      NOT NULL,
    email                 VARCHAR(254)      NOT NULL,
    phone                 VARCHAR(20)       NOT NULL,
    street                VARCHAR(200)      NOT NULL,
    postcode              VARCHAR(20)       NOT NULL,
    town                  VARCHAR(100)      NOT NULL,
    country               VARCHAR(100)      NOT NULL,
    arrival_time          SMALLINT UNSIGNED NOT NULL,
    message               TEXT              NOT NULL,
    -- Copy of the quote, house rules, promotion, options, payment instructions and rental
    -- terms in force when the booking was made (see domain::booking::Conditions).
    conditions            JSON              NOT NULL,
    stay_total            BIGINT UNSIGNED   NOT NULL,
    deposit               BIGINT UNSIGNED   NOT NULL,
    balance_due           DATE              NULL,
    paid_total            BIGINT UNSIGNED   NOT NULL DEFAULT 0,
    deposit_alert_sent_at DATETIME(3)       NULL,
    cancelled_at          DATETIME(3)       NULL,
    cancel_reason         VARCHAR(500)      NULL,
    created_at            DATETIME(3)       NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT bookings_reference_unique UNIQUE (reference),
    CONSTRAINT bookings_number_unique UNIQUE (cottage_id, reference_year, reference_number),
    CONSTRAINT bookings_token_unique UNIQUE (token),
    KEY bookings_dates (cottage_id, arrival, departure),
    CONSTRAINT bookings_cottage_fk FOREIGN KEY (cottage_id) REFERENCES cottages (id),
    CONSTRAINT bookings_dates_order CHECK (departure > arrival),
    CONSTRAINT bookings_status CHECK (status IN ('en_paiement', 'paiement_abandonne', 'confirmee',
                                                 'acompte_recu', 'soldee', 'annulee')),
    CONSTRAINT bookings_payment_mode CHECK (payment_mode IN ('offline', 'online'))
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE booking_payments (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    booking_id BIGINT UNSIGNED NOT NULL,
    amount     BIGINT UNSIGNED NOT NULL,
    paid_on    DATE            NOT NULL,
    method     VARCHAR(100)    NOT NULL,
    note       VARCHAR(500)    NOT NULL,
    author_id  BIGINT UNSIGNED NULL,
    created_at DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY booking_payments_booking (booking_id),
    CONSTRAINT booking_payments_booking_fk FOREIGN KEY (booking_id) REFERENCES bookings (id) ON DELETE CASCADE,
    CONSTRAINT booking_payments_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL,
    CONSTRAINT booking_payments_positive CHECK (amount > 0)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- History of each booking: creation, payments, messages, changes, cancellation.
CREATE TABLE booking_events (
    id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    booking_id  BIGINT UNSIGNED NOT NULL,
    description VARCHAR(500)    NOT NULL,
    author_id   BIGINT UNSIGNED NULL,
    at          DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY booking_events_booking (booking_id, at),
    CONSTRAINT booking_events_booking_fk FOREIGN KEY (booking_id) REFERENCES bookings (id) ON DELETE CASCADE,
    CONSTRAINT booking_events_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Booking attempts per IP address, for the rate limit; old lines are deleted as they go.
CREATE TABLE booking_attempts (
    id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    ip VARCHAR(45)     NOT NULL,
    at DATETIME(3)     NOT NULL,
    PRIMARY KEY (id),
    KEY booking_attempts_ip (ip, at)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Messages of a booking, for its detail screen; attachments are files of the private directory.
ALTER TABLE outgoing_messages
    ADD COLUMN booking_id  BIGINT UNSIGNED NULL,
    ADD COLUMN attachments JSON            NULL,
    ADD KEY outgoing_messages_booking (booking_id),
    ADD CONSTRAINT outgoing_messages_booking_fk FOREIGN KEY (booking_id) REFERENCES bookings (id) ON DELETE SET NULL;

-- Templates of the automatic messages (ADR-0021); the default texts are kept to be restored.
CREATE TABLE message_templates (
    kind            VARCHAR(40)  NOT NULL,
    channel         VARCHAR(10)  NOT NULL,
    lang            CHAR(2)      NOT NULL,
    subject         VARCHAR(200) NOT NULL,
    body            MEDIUMTEXT   NOT NULL,
    default_subject VARCHAR(200) NOT NULL,
    default_body    MEDIUMTEXT   NOT NULL,
    updated_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (kind, channel, lang),
    CONSTRAINT message_templates_channel CHECK (channel IN ('email', 'sms'))
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

INSERT INTO message_templates (kind, channel, lang, subject, body, default_subject, default_body)
SELECT kind, channel, lang, subject, body, subject, body FROM (
    SELECT 'booking_confirmed' AS kind, 'email' AS channel, 'fr' AS lang,
           'Votre réservation {{ sejour.reference }} est confirmée' AS subject,
           '<p>Bonjour {{ voyageur.prenom }},</p><p>Merci pour votre réservation ! Votre séjour du <strong>{{ sejour.arrivee }}</strong> au <strong>{{ sejour.depart }}</strong> ({{ sejour.nuits }} nuits, {{ sejour.voyageurs }} voyageurs) est confirmé.</p><p>Prix du séjour : {{ montants.total }}. Acompte de {{ montants.acompte }} à régler {{ montants.acompte_echeance }}, solde de {{ montants.solde }} à régler {{ montants.solde_echeance }}. Taxe de séjour à régler sur place : {{ montants.taxe_sejour }}.</p><h3>Comment payer</h3>{{ regles.paiement }}<p>Arrivée à partir de {{ regles.heure_arrivee }} (vous avez prévu {{ sejour.heure_arrivee }}), départ avant {{ regles.heure_depart }}.</p><p>Suivez votre réservation : {{ lien_suivi }}</p><p>À bientôt !</p>' AS body
    UNION ALL SELECT 'booking_confirmed', 'email', 'en',
           'Your booking {{ sejour.reference }} is confirmed',
           '<p>Hello {{ voyageur.prenom }},</p><p>Thank you for your booking! Your stay from <strong>{{ sejour.arrivee }}</strong> to <strong>{{ sejour.depart }}</strong> ({{ sejour.nuits }} nights, {{ sejour.voyageurs }} guests) is confirmed.</p><p>Price of the stay: {{ montants.total }}. Deposit of {{ montants.acompte }} to pay {{ montants.acompte_echeance }}, balance of {{ montants.solde }} to pay {{ montants.solde_echeance }}. Tourist tax to pay on arrival: {{ montants.taxe_sejour }}.</p><h3>How to pay</h3>{{ regles.paiement }}<p>Arrival from {{ regles.heure_arrivee }} (you planned {{ sejour.heure_arrivee }}), departure before {{ regles.heure_depart }}.</p><p>Follow your booking: {{ lien_suivi }}</p><p>See you soon!</p>'
    UNION ALL SELECT 'booking_confirmed', 'sms', 'fr', '',
           'Réservation {{ sejour.reference }} confirmée du {{ sejour.arrivee }} au {{ sejour.depart }}. Acompte : {{ montants.acompte }}. Détails envoyés par e-mail.'
    UNION ALL SELECT 'booking_confirmed', 'sms', 'en', '',
           'Booking {{ sejour.reference }} confirmed from {{ sejour.arrivee }} to {{ sejour.depart }}. Deposit: {{ montants.acompte }}. Details sent by email.'
    UNION ALL SELECT 'new_booking', 'email', 'fr',
           'Nouvelle réservation {{ sejour.reference }} du {{ sejour.arrivee }} au {{ sejour.depart }}',
           '<p>Nouvelle réservation directe : <strong>{{ voyageur.prenom }} {{ voyageur.nom }}</strong>, du {{ sejour.arrivee }} au {{ sejour.depart }} ({{ sejour.nuits }} nuits, {{ sejour.voyageurs }} voyageurs, arrivée prévue à {{ sejour.heure_arrivee }}).</p><p>Prix du séjour : {{ montants.total }}, acompte attendu : {{ montants.acompte }}.</p><p>E-mail : {{ voyageur.email }}, téléphone : {{ voyageur.telephone }}.</p><p>Message : {{ voyageur.message }}</p><p><strong>Pensez à bloquer ces dates sur Airbnb et Booking.</strong></p><p>Voir la réservation : {{ lien_admin }}</p>'
    UNION ALL SELECT 'new_booking', 'sms', 'fr', '',
           'Nouvelle réservation {{ sejour.reference }} : {{ voyageur.prenom }} {{ voyageur.nom }}, du {{ sejour.arrivee }} au {{ sejour.depart }}. Bloquez ces dates sur Airbnb et Booking.'
    UNION ALL SELECT 'deposit_late', 'email', 'fr',
           'Acompte non reçu pour la réservation {{ sejour.reference }}',
           '<p>L''acompte de {{ montants.acompte }} de la réservation {{ sejour.reference }} ({{ voyageur.prenom }} {{ voyageur.nom }}, du {{ sejour.arrivee }} au {{ sejour.depart }}) n''est pas encore marqué reçu.</p><p>Contactez le voyageur ({{ voyageur.email }}, {{ voyageur.telephone }}) ou annulez la réservation : {{ lien_admin }}</p>'
    UNION ALL SELECT 'deposit_late', 'sms', 'fr', '',
           'Acompte non reçu : réservation {{ sejour.reference }} de {{ voyageur.prenom }} {{ voyageur.nom }} ({{ sejour.arrivee }}).'
) AS seed;
