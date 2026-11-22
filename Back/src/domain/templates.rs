//! Message templates written by the owners (F007, then F016; ADR-0021): which variables each
//! template may use, the check done when saving, and the sandboxed rendering.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use minijinja::value::{Serde, Value};
use minijinja::{AutoEscape, Environment, UndefinedBehavior};

use crate::domain::messaging::Channel;
use crate::domain::page::Lang;

/// Longest subject.
pub const MAX_SUBJECT: usize = 200;
/// Longest body (email HTML or SMS text).
pub const MAX_BODY: usize = 20_000;
/// Longest SMS text, before rendering.
pub const MAX_SMS: usize = 1_000;

/// Who receives the messages of a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recipient {
    Guest,
    Owners,
}

/// The automatic messages the owners can edit (F007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TemplateKind {
    /// To the guest, as soon as the booking is made.
    BookingConfirmed,
    /// To the owners, as soon as a booking is made.
    NewBooking,
    /// To the owners, when the deposit is late.
    DepositLate,
}

/// A variable a template may use, with the label of its button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Variable {
    pub name: &'static str,
    pub label: &'static str,
}

const fn var(name: &'static str, label: &'static str) -> Variable {
    Variable { name, label }
}

/// Variables every template may use.
const COMMON: [Variable; 18] = [
    var("voyageur.prenom", "Prénom du voyageur"),
    var("voyageur.nom", "Nom du voyageur"),
    var("sejour.reference", "Référence"),
    var("sejour.arrivee", "Date d'arrivée"),
    var("sejour.depart", "Date de départ"),
    var("sejour.nuits", "Nombre de nuits"),
    var("sejour.voyageurs", "Nombre de voyageurs"),
    var("sejour.heure_arrivee", "Heure d'arrivée prévue"),
    var("montants.total", "Prix du séjour"),
    var("montants.acompte", "Acompte"),
    var("montants.acompte_echeance", "Échéance de l'acompte"),
    var("montants.solde", "Solde"),
    var("montants.solde_echeance", "Échéance du solde"),
    var("montants.taxe_sejour", "Taxe de séjour"),
    var("regles.heure_arrivee", "Arrivée à partir de"),
    var("regles.heure_depart", "Départ avant"),
    var("regles.paiement", "Comment payer"),
    var("lien_suivi", "Lien de suivi"),
];

/// Variables only the owners' templates may use.
const OWNERS_ONLY: [Variable; 4] = [
    var("voyageur.email", "E-mail du voyageur"),
    var("voyageur.telephone", "Téléphone du voyageur"),
    var("voyageur.message", "Message du voyageur"),
    var("lien_admin", "Lien vers la réservation (admin)"),
];

impl TemplateKind {
    pub const ALL: [Self; 3] = [Self::BookingConfirmed, Self::NewBooking, Self::DepositLate];

    /// Stored name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BookingConfirmed => "booking_confirmed",
            Self::NewBooking => "new_booking",
            Self::DepositLate => "deposit_late",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }

    /// Name shown to the owners.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::BookingConfirmed => "Réservation confirmée",
            Self::NewBooking => "Nouvelle réservation",
            Self::DepositLate => "Acompte non reçu",
        }
    }

    #[must_use]
    pub fn recipient(self) -> Recipient {
        match self {
            Self::BookingConfirmed => Recipient::Guest,
            Self::NewBooking | Self::DepositLate => Recipient::Owners,
        }
    }

    /// Languages the template is written in: the owners always read French.
    #[must_use]
    pub fn languages(self) -> &'static [Lang] {
        match self.recipient() {
            Recipient::Guest => &Lang::ALL,
            Recipient::Owners => &[Lang::Fr],
        }
    }

    /// Variables the template may use, in the order of the buttons.
    #[must_use]
    pub fn variables(self) -> Vec<Variable> {
        let mut variables = COMMON.to_vec();
        if self.recipient() == Recipient::Owners {
            variables.extend(OWNERS_ONLY);
        }
        variables
    }
}

/// Why a template cannot be saved; the text is shown to the owners.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TemplateError {
    #[error("variable inconnue : {0}")]
    UnknownVariable(String),
    #[error("seules les variables {{{{ nom }}}} sont permises : « {0} »")]
    Syntax(String),
    #[error("objet : {MAX_SUBJECT} caractères au maximum")]
    SubjectTooLong,
    #[error("texte : {0} caractères au maximum")]
    BodyTooLong(usize),
    #[error("l'objet de l'e-mail est obligatoire quand le texte est rempli")]
    SubjectMissing,
}

/// The variables written in `source`, or the first construct that is not a plain variable.
fn scan(source: &str) -> Result<Vec<String>, TemplateError> {
    for forbidden in ["{%", "{#", "%}", "#}"] {
        if source.contains(forbidden) {
            return Err(TemplateError::Syntax(forbidden.to_owned()));
        }
    }
    let mut names = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            return Err(TemplateError::Syntax(excerpt(&rest[start..])));
        };
        let name = after[..end].trim();
        let plain = !name.is_empty()
            && name.split('.').count() <= 2
            && name.split('.').all(|part| {
                !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
            });
        if !plain {
            return Err(TemplateError::Syntax(excerpt(
                &rest[start..start + 4 + end],
            )));
        }
        names.push(name.to_owned());
        rest = &after[end + 2..];
    }
    if rest.contains("}}") {
        return Err(TemplateError::Syntax("}}".to_owned()));
    }
    Ok(names)
}

fn excerpt(text: &str) -> String {
    text.chars().take(40).collect()
}

/// Checks a template before it is saved: plain variables of the allow-list only, lengths.
///
/// # Errors
///
/// Every problem found.
pub fn check(
    kind: TemplateKind,
    channel: Channel,
    subject: &str,
    body: &str,
) -> Result<(), Vec<TemplateError>> {
    let mut errors = Vec::new();
    let allowed = kind.variables();
    let limit = match channel {
        Channel::Email => MAX_BODY,
        Channel::Sms => MAX_SMS,
    };
    if body.chars().count() > limit {
        errors.push(TemplateError::BodyTooLong(limit));
    }
    if channel == Channel::Email {
        if subject.chars().count() > MAX_SUBJECT {
            errors.push(TemplateError::SubjectTooLong);
        }
        if subject.trim().is_empty() && !html_to_text(body).trim().is_empty() {
            errors.push(TemplateError::SubjectMissing);
        }
    }
    for source in [subject, body] {
        match scan(source) {
            Ok(names) => {
                for name in names {
                    let error = TemplateError::UnknownVariable(name.clone());
                    if !allowed.iter().any(|variable| variable.name == name)
                        && !errors.contains(&error)
                    {
                        errors.push(error);
                    }
                }
            }
            Err(error) => {
                if !errors.contains(&error) {
                    errors.push(error);
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Values of a message, already formatted in its language.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageValues {
    /// `name` → text, for every variable except `regles.paiement`.
    pub texts: BTreeMap<&'static str, String>,
    /// Payment instructions, sanitised HTML.
    pub payment_html: String,
}

/// Name of the variable holding the payment instructions (HTML).
const PAYMENT: &str = "regles.paiement";

impl MessageValues {
    /// Context of the template: nested objects (`voyageur.prenom`), the payment
    /// instructions as trusted HTML in emails and as text elsewhere.
    fn context(&self, html: bool) -> Value {
        let mut root: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
        let mut top: BTreeMap<String, Value> = BTreeMap::new();
        let mut put = |name: &str, value: Value| match name.split_once('.') {
            Some((group, key)) => {
                root.entry(group.to_owned())
                    .or_default()
                    .insert(key.to_owned(), value);
            }
            None => {
                top.insert(name.to_owned(), value);
            }
        };
        for (name, text) in &self.texts {
            put(name, Value::from(text.as_str()));
        }
        put(
            PAYMENT,
            if html {
                Value::from_safe_string(self.payment_html.clone())
            } else {
                Value::from(html_to_text(&self.payment_html))
            },
        );
        let mut context: BTreeMap<String, Value> = top;
        for (group, values) in root {
            context.insert(group, Value::from(Serde(values)));
        }
        Value::from(Serde(context))
    }
}

/// Renders a checked template in a sandbox: no loader, no filters nor functions, only the
/// given values; values are escaped when `html`.
///
/// # Errors
///
/// A variable without value, or a template that was not checked.
pub fn render(source: &str, values: &MessageValues, html: bool) -> anyhow::Result<String> {
    let mut environment = Environment::new();
    environment.set_undefined_behavior(UndefinedBehavior::Strict);
    environment.set_auto_escape_callback(move |_| {
        if html {
            AutoEscape::Html
        } else {
            AutoEscape::None
        }
    });
    let rendered = environment
        .render_str(source, values.context(html))
        .map_err(|error| anyhow::anyhow!("cannot render a template: {error}"))?;
    Ok(if html {
        rendered
    } else {
        rendered
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect()
    })
}

/// Plain text of sanitised HTML: paragraphs and list items on their own lines.
#[must_use]
pub fn html_to_text(html: &str) -> String {
    let mut text = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        text.push_str(&decode(&rest[..start]));
        let Some(end) = rest[start..].find('>') else {
            rest = "";
            break;
        };
        let tag = rest[start + 1..start + end].trim().to_ascii_lowercase();
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        let closing = tag.starts_with('/');
        match (name.as_str(), closing) {
            ("br", _) | ("li", true) => text.push('\n'),
            ("li", false) => text.push_str("- "),
            ("p" | "h1" | "h2" | "h3" | "h4" | "ul" | "ol", true) => text.push_str("\n\n"),
            _ => {}
        }
        rest = &rest[start + end + 1..];
    }
    text.push_str(&decode(rest));
    let mut lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    lines.dedup_by(|a, b| a.is_empty() && b.is_empty());
    lines.join("\n").trim().to_owned()
}

/// Decodes the named entities of the sanitiser and the numeric ones (`&#x2f;`, `&#39;`).
fn decode(text: &str) -> String {
    let mut decoded = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let entity = after
            .find(';')
            .filter(|end| *end <= 8)
            .map(|end| (&after[..end], end));
        let character = entity.and_then(|(name, _)| match name {
            "nbsp" => Some(' '),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "amp" => Some('&'),
            _ => name
                .strip_prefix("#x")
                .or_else(|| name.strip_prefix("#X"))
                .map(|hex| u32::from_str_radix(hex, 16))
                .or_else(|| name.strip_prefix('#').map(str::parse))
                .and_then(Result::ok)
                .and_then(char::from_u32),
        });
        if let (Some(character), Some((_, end))) = (character, entity) {
            decoded.push(character);
            rest = &after[end + 1..];
        } else {
            decoded.push('&');
            rest = after;
        }
    }
    decoded.push_str(rest);
    decoded
}

/// Escapes a text for HTML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fixed layout of every email: the cottage name on top, the body, the site address below.
/// Inline styles only, readable in every mail client.
#[must_use]
pub fn email_layout(cottage: &str, body_html: &str, site_url: &str) -> String {
    let mut html = String::new();
    let cottage = escape(cottage);
    let site = escape(site_url);
    let _ = write!(
        html,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{cottage}</title></head>\
         <body style=\"margin:0;padding:0;background:#f1f5f9;\">\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"background:#f1f5f9;\"><tr><td align=\"center\" style=\"padding:24px 12px;\">\
         <table role=\"presentation\" width=\"600\" cellpadding=\"0\" cellspacing=\"0\" \
         style=\"max-width:600px;width:100%;background:#ffffff;border-radius:8px;\">\
         <tr><td style=\"background:#1d4ed8;color:#ffffff;padding:16px 24px;border-radius:8px 8px 0 0;\
         font-family:Arial,Helvetica,sans-serif;font-size:20px;font-weight:bold;\">{cottage}</td></tr>\
         <tr><td style=\"padding:24px;font-family:Arial,Helvetica,sans-serif;font-size:15px;\
         line-height:1.5;color:#0f172a;\">{body_html}</td></tr>\
         <tr><td style=\"padding:16px 24px;border-top:1px solid #e2e8f0;\
         font-family:Arial,Helvetica,sans-serif;font-size:12px;color:#64748b;\">\
         <a href=\"{site}\" style=\"color:#1d4ed8;\">{site}</a></td></tr>\
         </table></td></tr></table></body></html>"
    );
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> MessageValues {
        let mut texts = BTreeMap::new();
        texts.insert("voyageur.prenom", "Léa <script>".to_owned());
        texts.insert("sejour.reference", "LPG-2027-0001".to_owned());
        texts.insert(
            "lien_suivi",
            "https://example.test/reservation/abc".to_owned(),
        );
        MessageValues {
            texts,
            payment_html: "<p>Virement &amp; chèque</p>".to_owned(),
        }
    }

    #[test]
    fn f007_s12_unknown_variable_is_refused_by_name() {
        let errors = check(
            TemplateKind::BookingConfirmed,
            Channel::Email,
            "Réservation {{ sejour.reference }}",
            "<p>Bonjour {{ voyageur.prenon }}, {{voyageur.prenom}}</p>",
        )
        .unwrap_err();
        assert_eq!(
            errors,
            vec![TemplateError::UnknownVariable("voyageur.prenon".to_owned())]
        );
    }

    #[test]
    fn owners_variables_are_refused_in_guest_templates() {
        let errors = check(
            TemplateKind::BookingConfirmed,
            Channel::Sms,
            "",
            "{{ voyageur.telephone }}",
        )
        .unwrap_err();
        assert_eq!(
            errors,
            vec![TemplateError::UnknownVariable(
                "voyageur.telephone".to_owned()
            )]
        );
        assert!(
            check(
                TemplateKind::NewBooking,
                Channel::Sms,
                "",
                "{{ voyageur.telephone }}"
            )
            .is_ok()
        );
    }

    #[test]
    fn tags_filters_and_expressions_are_refused() {
        for body in [
            "{% include 'x' %}",
            "{{ voyageur.prenom | safe }}",
            "{{ voyageur.prenom ~ 'x' }}",
            "{{ range(10) }}",
            "{# comment #}",
            "{{ voyageur.prenom",
        ] {
            let errors = check(TemplateKind::NewBooking, Channel::Sms, "", body).unwrap_err();
            assert!(
                matches!(errors[0], TemplateError::Syntax(_)),
                "{body}: {errors:?}"
            );
        }
    }

    #[test]
    fn an_email_with_a_body_needs_a_subject() {
        let errors = check(TemplateKind::NewBooking, Channel::Email, " ", "<p>x</p>").unwrap_err();
        assert_eq!(errors, vec![TemplateError::SubjectMissing]);
        assert!(check(TemplateKind::NewBooking, Channel::Email, "", "").is_ok());
    }

    #[test]
    fn guest_values_are_escaped_in_emails_and_payment_stays_html() {
        let html = render(
            "<p>Bonjour {{ voyageur.prenom }}</p>{{ regles.paiement }}",
            &values(),
            true,
        )
        .unwrap();
        assert_eq!(
            html,
            "<p>Bonjour Léa &lt;script&gt;</p><p>Virement &amp; chèque</p>"
        );
        let text = render(
            "{{ voyageur.prenom }} : {{ regles.paiement }}",
            &values(),
            false,
        )
        .unwrap();
        assert_eq!(text, "Léa <script> : Virement & chèque");
    }

    #[test]
    fn rendering_has_no_filters_nor_loader() {
        assert!(render("{{ voyageur.prenom | upper }}", &values(), false).is_err());
        assert!(render("{% include 'secret.txt' %}", &values(), false).is_err());
        assert!(render("{{ voyageur.inconnu }}", &values(), false).is_err());
    }

    #[test]
    fn html_becomes_readable_text() {
        assert_eq!(
            html_to_text(
                "<h2>Titre</h2><p>Un<br>deux &amp; trois</p><ul><li>a</li><li>b</li></ul><p>fin</p>"
            ),
            "Titre\n\nUn\ndeux & trois\n\n- a\n- b\n\nfin"
        );
    }

    #[test]
    fn kinds_have_stable_names() {
        for kind in TemplateKind::ALL {
            assert_eq!(TemplateKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(TemplateKind::DepositLate.languages(), &[Lang::Fr]);
    }
}
