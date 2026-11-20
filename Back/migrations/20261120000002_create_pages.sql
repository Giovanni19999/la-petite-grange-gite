-- F002: pages built from blocks, draft / published content and published versions.
-- Content is JSON (see domain::page::PageContent); timestamps are UTC.

CREATE TABLE pages (
    id                BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
    is_home           BOOLEAN         NOT NULL DEFAULT FALSE,
    draft             JSON            NOT NULL,
    draft_version     INT UNSIGNED    NOT NULL DEFAULT 1,
    published         JSON            NULL,
    published_at      DATETIME(3)     NULL,
    -- Copies of the published slugs, for lookups and uniqueness.
    published_slug_fr VARCHAR(80)     NULL,
    published_slug_en VARCHAR(80)     NULL,
    created_at        DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at        DATETIME(3)     NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    PRIMARY KEY (id),
    CONSTRAINT pages_published_slug_fr_unique UNIQUE (published_slug_fr),
    CONSTRAINT pages_published_slug_en_unique UNIQUE (published_slug_en)
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

CREATE TABLE page_versions (
    page_id      BIGINT UNSIGNED NOT NULL,
    number       INT UNSIGNED    NOT NULL,
    content      JSON            NOT NULL,
    published_at DATETIME(3)     NOT NULL,
    author_id    BIGINT UNSIGNED NULL,
    PRIMARY KEY (page_id, number),
    CONSTRAINT page_versions_page_fk FOREIGN KEY (page_id) REFERENCES pages (id) ON DELETE CASCADE,
    CONSTRAINT page_versions_author_fk FOREIGN KEY (author_id) REFERENCES users (id) ON DELETE SET NULL
) ENGINE = InnoDB DEFAULT CHARSET = utf8mb4 COLLATE = utf8mb4_unicode_ci;

-- Default pages (seed, ADR-0010): generic texts the owners replace from the admin.
CREATE TEMPORARY TABLE seed_pages (
    is_home BOOLEAN NOT NULL,
    content JSON    NOT NULL
);

INSERT INTO seed_pages (is_home, content) VALUES
(TRUE, '{"settings":{"slug":{"fr":"accueil","en":"home"},"title":{"fr":"La Petite Grange","en":"La Petite Grange"},"meta_description":{"fr":"Gîte classé 3 étoiles à la campagne : réservez en direct.","en":"3-star holiday cottage in the countryside: book direct."},"in_menu":true,"menu_order":0},"blocks":[{"id":"hero","type":"hero","title":{"fr":"La Petite Grange","en":"La Petite Grange"},"subtitle":{"fr":"Gîte classé 3 étoiles, au calme de la campagne","en":"3-star holiday cottage in the quiet countryside"},"button_label":{"fr":"Réserver","en":"Book"},"button_link":"#reserver","align":"center","height":"large","darken":30},{"id":"intro","type":"text","body":{"fr":"<p>Bienvenue ! Ce texte de présentation est à remplacer depuis l’espace administrateur.</p>","en":"<p>Welcome! This introduction is to be replaced from the admin.</p>"},"width":"narrow","align":"center"},{"id":"atouts","type":"columns","items":[{"title":{"fr":"Au calme","en":"Peaceful"},"body":{"fr":"<p>Un cadre reposant.</p>","en":"<p>A restful setting.</p>"}},{"title":{"fr":"Tout équipé","en":"Fully equipped"},"body":{"fr":"<p>Pour 5 personnes.</p>","en":"<p>For 5 guests.</p>"}},{"title":{"fr":"Réservation directe","en":"Direct booking"},"body":{"fr":"<p>Sans commission.</p>","en":"<p>No commission.</p>"}}]}]}'),
(FALSE, '{"settings":{"slug":{"fr":"le-gite","en":"the-cottage"},"title":{"fr":"Le gîte","en":"The cottage"},"meta_description":{"fr":"Chambres, équipements et photos du gîte.","en":"Bedrooms, amenities and photos of the cottage."},"in_menu":true,"menu_order":1},"blocks":[{"id":"description","type":"text_image","body":{"fr":"<h2>Le gîte</h2><p>Description du gîte à compléter.</p>","en":"<h2>The cottage</h2><p>Description of the cottage to be completed.</p>"},"image_position":"right","ratio":"50-50"}]}'),
(FALSE, '{"settings":{"slug":{"fr":"la-region","en":"the-area"},"title":{"fr":"La région","en":"The area"},"meta_description":{"fr":"Activités, balades et sites à visiter autour du gîte.","en":"Activities, walks and places to visit near the cottage."},"in_menu":true,"menu_order":2},"blocks":[{"id":"region","type":"text","body":{"fr":"<p>Activités, balades, restaurants et sites à visiter : texte à compléter.</p>","en":"<p>Activities, walks, restaurants and places to visit: to be completed.</p>"},"width":"wide","align":"left"}]}'),
(FALSE, '{"settings":{"slug":{"fr":"acces","en":"getting-here"},"title":{"fr":"Accès","en":"Getting here"},"meta_description":{"fr":"Comment venir au gîte.","en":"How to get to the cottage."},"in_menu":true,"menu_order":3},"blocks":[{"id":"acces","type":"text","body":{"fr":"<p>Adresse et itinéraire : texte à compléter.</p>","en":"<p>Address and directions: to be completed.</p>"},"width":"narrow","align":"left"}]}'),
(FALSE, '{"settings":{"slug":{"fr":"mentions-legales","en":"legal-notice"},"title":{"fr":"Mentions légales","en":"Legal notice"},"meta_description":{"fr":"","en":""},"in_menu":false,"menu_order":10},"blocks":[{"id":"mentions","type":"text","body":{"fr":"<p>Éditeur, hébergeur et numéro d’enregistrement du meublé : à compléter.</p>","en":""},"width":"narrow","align":"left"}]}'),
(FALSE, '{"settings":{"slug":{"fr":"confidentialite","en":"privacy"},"title":{"fr":"Confidentialité","en":"Privacy"},"meta_description":{"fr":"","en":""},"in_menu":false,"menu_order":11},"blocks":[{"id":"confidentialite","type":"text","body":{"fr":"<p>Politique de confidentialité : à compléter.</p>","en":""},"width":"narrow","align":"left"}]}'),
(FALSE, '{"settings":{"slug":{"fr":"conditions-de-location","en":"rental-terms"},"title":{"fr":"Conditions de location","en":"Rental terms"},"meta_description":{"fr":"","en":""},"in_menu":false,"menu_order":12},"blocks":[{"id":"conditions","type":"text","body":{"fr":"<p>Conditions générales de location : à compléter.</p>","en":""},"width":"narrow","align":"left"}]}');

INSERT INTO pages (is_home, draft, draft_version, published, published_at, published_slug_fr, published_slug_en)
SELECT is_home, content, 1, content, CURRENT_TIMESTAMP(3),
       JSON_UNQUOTE(JSON_EXTRACT(content, '$.settings.slug.fr')),
       JSON_UNQUOTE(JSON_EXTRACT(content, '$.settings.slug.en'))
FROM seed_pages;

INSERT INTO page_versions (page_id, number, content, published_at, author_id)
SELECT id, 1, published, published_at, NULL FROM pages;

DROP TEMPORARY TABLE seed_pages;
