//! Message templates use cases (F007, ADR-0021): list, edit, restore and preview the
//! templates, and turn a template into the content of a message.

use std::fmt;
use std::sync::Arc;

use crate::application::health::BoxFuture;
use crate::domain::messaging::{Channel, Content};
use crate::domain::page::{Lang, sanitize_rich_text};
use crate::domain::templates::{
    MessageValues, TemplateError, TemplateKind, check, email_layout, html_to_text, render,
};

/// A template as stored, with its default text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTemplate {
    pub kind: TemplateKind,
    pub channel: Channel,
    pub lang: Lang,
    /// Empty for an SMS.
    pub subject: String,
    /// Sanitised HTML for an email, plain text for an SMS; empty: not sent.
    pub body: String,
    pub default_subject: String,
    pub default_body: String,
}

/// Port: the templates in the database.
pub trait TemplateRepository: Send + Sync {
    fn list(&self) -> BoxFuture<'_, anyhow::Result<Vec<StoredTemplate>>>;
    /// False if there is no such template.
    fn save<'a>(
        &'a self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
        subject: &'a str,
        body: &'a str,
    ) -> BoxFuture<'a, anyhow::Result<bool>>;
    /// Puts the default text back; false if there is no such template.
    fn restore(
        &self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
    ) -> BoxFuture<'_, anyhow::Result<bool>>;
}

/// Why a template use case is refused.
#[derive(Debug, thiserror::Error)]
pub enum TemplatesError {
    #[error("modèle invalide")]
    Invalid(Vec<TemplateError>),
    #[error("modèle introuvable")]
    NotFound,
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Where the layout of the emails points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub cottage: String,
    pub site_url: String,
}

/// A rendered message, for the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub subject: String,
    /// Full email HTML (with the layout); `None` for an SMS.
    pub html: Option<String>,
    pub text: String,
}

/// The template of `kind` and `channel` to use in `lang`: French when the text in `lang` is
/// empty; `None` when the French text is empty too (the message is not sent).
#[must_use]
pub fn pick(
    templates: &[StoredTemplate],
    kind: TemplateKind,
    channel: Channel,
    lang: Lang,
) -> Option<&StoredTemplate> {
    let find = |lang: Lang| {
        templates.iter().find(|template| {
            template.kind == kind
                && template.channel == channel
                && template.lang == lang
                && !template.body.trim().is_empty()
        })
    };
    find(lang).or_else(|| find(Lang::Fr))
}

/// Content of a message from a template and the values of a booking.
///
/// # Errors
///
/// A value missing for a variable of the template.
pub fn content(
    template: &StoredTemplate,
    values: &MessageValues,
    layout: &Layout,
) -> anyhow::Result<Content> {
    Ok(match template.channel {
        Channel::Email => {
            let body = render(&template.body, values, true)?;
            Content::Email {
                subject: render(&template.subject, values, false)?
                    .replace('\n', " ")
                    .trim()
                    .to_owned(),
                text: html_to_text(&body),
                html: Some(email_layout(&layout.cottage, &body, &layout.site_url)),
                attachments: Vec::new(),
            }
        }
        Channel::Sms => Content::Sms {
            text: render(&template.body, values, false)?.trim().to_owned(),
        },
    })
}

/// Template use cases.
#[derive(Clone)]
pub struct TemplatesService {
    templates: Arc<dyn TemplateRepository>,
}

impl fmt::Debug for TemplatesService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TemplatesService").finish_non_exhaustive()
    }
}

impl TemplatesService {
    #[must_use]
    pub fn new(templates: Arc<dyn TemplateRepository>) -> Self {
        Self { templates }
    }

    /// Every template.
    ///
    /// # Errors
    ///
    /// Infrastructure errors.
    pub async fn list(&self) -> anyhow::Result<Vec<StoredTemplate>> {
        self.templates.list().await
    }

    /// The subject and body cleaned (HTML sanitised for an email), or every problem.
    fn cleaned(
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
        subject: &str,
        body: &str,
    ) -> Result<(String, String), TemplatesError> {
        if !kind.languages().contains(&lang) {
            return Err(TemplatesError::NotFound);
        }
        let (subject, body) = match channel {
            Channel::Email => {
                let body = sanitize_rich_text(body);
                let body = if html_to_text(&body).trim().is_empty() {
                    String::new()
                } else {
                    body
                };
                (subject.trim().to_owned(), body)
            }
            Channel::Sms => (String::new(), body.trim().to_owned()),
        };
        check(kind, channel, &subject, &body).map_err(TemplatesError::Invalid)?;
        Ok((subject, body))
    }

    /// Replaces the text of a template.
    ///
    /// # Errors
    ///
    /// Unknown template, unknown variable or forbidden construct, infrastructure.
    pub async fn save(
        &self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
        subject: &str,
        body: &str,
    ) -> Result<(), TemplatesError> {
        let (subject, body) = Self::cleaned(kind, channel, lang, subject, body)?;
        if self
            .templates
            .save(kind, channel, lang, &subject, &body)
            .await?
        {
            Ok(())
        } else {
            Err(TemplatesError::NotFound)
        }
    }

    /// Puts the default text of a template back.
    ///
    /// # Errors
    ///
    /// Unknown template, infrastructure.
    pub async fn restore(
        &self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
    ) -> Result<(), TemplatesError> {
        if self.templates.restore(kind, channel, lang).await? {
            Ok(())
        } else {
            Err(TemplatesError::NotFound)
        }
    }

    /// Renders an edited (not yet saved) template with sample values.
    ///
    /// # Errors
    ///
    /// Same refusals as saving.
    pub fn preview(
        &self,
        kind: TemplateKind,
        channel: Channel,
        lang: Lang,
        subject: &str,
        body: &str,
        layout: &Layout,
    ) -> Result<Rendered, TemplatesError> {
        let (subject, body) = Self::cleaned(kind, channel, lang, subject, body)?;
        let template = StoredTemplate {
            kind,
            channel,
            lang,
            subject,
            body,
            default_subject: String::new(),
            default_body: String::new(),
        };
        Ok(
            match content(&template, &sample_values(lang, &layout.site_url), layout)? {
                Content::Email {
                    subject,
                    text,
                    html,
                    ..
                } => Rendered {
                    subject,
                    html,
                    text,
                },
                Content::Sms { text } => Rendered {
                    subject: String::new(),
                    html: None,
                    text,
                },
            },
        )
    }
}

/// Values of an imaginary booking, for the previews.
#[must_use]
pub fn sample_values(lang: Lang, site_url: &str) -> MessageValues {
    let english = lang == Lang::En;
    let pick = |fr: &str, en: &str| if english { en } else { fr }.to_owned();
    let site = site_url.trim_end_matches('/');
    let texts = [
        ("voyageur.prenom", "Camille".to_owned()),
        ("voyageur.nom", "Exemple".to_owned()),
        ("voyageur.email", "camille@example.test".to_owned()),
        ("voyageur.telephone", "+33600000000".to_owned()),
        (
            "voyageur.message",
            pick("Nous arriverons avec un bébé.", "We will come with a baby."),
        ),
        ("sejour.reference", "LPG-2027-0001".to_owned()),
        ("sejour.arrivee", pick("10 juillet 2027", "10 July 2027")),
        ("sejour.depart", pick("14 juillet 2027", "14 July 2027")),
        ("sejour.nuits", "4".to_owned()),
        ("sejour.voyageurs", "3".to_owned()),
        ("sejour.heure_arrivee", "18:00".to_owned()),
        ("montants.total", pick("520,00 €", "€520.00")),
        ("montants.acompte", pick("260,00 €", "€260.00")),
        (
            "montants.acompte_echeance",
            pick("à la réservation", "at booking"),
        ),
        ("montants.solde", pick("260,00 €", "€260.00")),
        (
            "montants.solde_echeance",
            pick("le 10 juin 2027", "on 10 June 2027"),
        ),
        ("montants.taxe_sejour", pick("12,00 €", "€12.00")),
        ("regles.heure_arrivee", "16:00".to_owned()),
        ("regles.heure_depart", "10:00".to_owned()),
        ("lien_suivi", format!("{site}/reservation/exemple")),
        ("lien_admin", format!("{site}/admin/reservations/1")),
    ];
    MessageValues {
        texts: texts.into_iter().collect(),
        payment_html: pick(
            "<p>(Les instructions de paiement des règles du gîte.)</p>",
            "<p>(The payment instructions of the house rules.)</p>",
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn template(lang: Lang, channel: Channel, body: &str) -> StoredTemplate {
        StoredTemplate {
            kind: TemplateKind::BookingConfirmed,
            channel,
            lang,
            subject: "Réservation {{ sejour.reference }}".to_owned(),
            body: body.to_owned(),
            default_subject: String::new(),
            default_body: String::new(),
        }
    }

    #[test]
    fn f007_s13_english_falls_back_to_french_when_empty() {
        let templates = vec![
            template(Lang::Fr, Channel::Sms, "Bonjour"),
            template(Lang::En, Channel::Sms, " "),
            template(Lang::En, Channel::Email, "<p>Hello</p>"),
        ];
        let sms = pick(
            &templates,
            TemplateKind::BookingConfirmed,
            Channel::Sms,
            Lang::En,
        );
        assert_eq!(sms.unwrap().lang, Lang::Fr);
        let email = pick(
            &templates,
            TemplateKind::BookingConfirmed,
            Channel::Email,
            Lang::En,
        );
        assert_eq!(email.unwrap().lang, Lang::En);
        assert!(pick(&templates, TemplateKind::NewBooking, Channel::Sms, Lang::Fr).is_none());
    }

    #[test]
    fn emails_get_the_layout_and_a_text_version() {
        let layout = Layout {
            cottage: "La Petite Grange".to_owned(),
            site_url: "https://example.test".to_owned(),
        };
        let content = content(
            &template(
                Lang::Fr,
                Channel::Email,
                "<p>Bonjour <strong>{{ voyageur.prenom }}</strong></p>",
            ),
            &sample_values(Lang::Fr, &layout.site_url),
            &layout,
        )
        .unwrap();
        let Content::Email {
            subject,
            text,
            html,
            ..
        } = content
        else {
            panic!("an email");
        };
        assert_eq!(subject, "Réservation LPG-2027-0001");
        assert_eq!(text, "Bonjour Camille");
        let html = html.unwrap();
        assert!(html.contains("<strong>Camille</strong>"));
        assert!(html.contains("La Petite Grange"));
    }

    #[test]
    fn saving_sanitises_the_html_and_refuses_unknown_variables() {
        let (_, body) = TemplatesService::cleaned(
            TemplateKind::NewBooking,
            Channel::Email,
            Lang::Fr,
            "Objet",
            "<p onclick=\"x()\">Hi</p><script>alert(1)</script>",
        )
        .unwrap();
        assert_eq!(body, "<p>Hi</p>");
        assert!(matches!(
            TemplatesService::cleaned(
                TemplateKind::NewBooking,
                Channel::Email,
                Lang::En,
                "Objet",
                "<p>Hi</p>"
            ),
            Err(TemplatesError::NotFound)
        ));
    }
}
