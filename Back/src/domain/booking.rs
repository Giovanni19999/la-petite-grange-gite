//! Direct bookings (F007, ADR-0020): statuses, the guest's details, the copy of the
//! conditions in force, payments recorded by the owners.

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};

use crate::domain::house_rules::{HouseRules, SLOT_MINUTES, clock_time};
use crate::domain::messaging::{valid_email, valid_phone};
use crate::domain::offers::{Promotion, StayOption};
use crate::domain::page::{Lang, Localized};

/// Last arrival time a guest may choose (23:30).
const LAST_ARRIVAL: u16 = 23 * 60 + 30;
/// Longest message of a guest to the owners.
pub const MAX_MESSAGE: usize = 2_000;
/// Longest cancellation reason or payment note.
pub const MAX_NOTE: usize = 500;

/// Where a booking stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BookingStatus {
    /// Confirmed, deposit expected.
    Confirmed,
    DepositReceived,
    /// Everything paid.
    Paid,
    Cancelled,
}

impl BookingStatus {
    pub const ALL: [Self; 4] = [
        Self::Confirmed,
        Self::DepositReceived,
        Self::Paid,
        Self::Cancelled,
    ];

    /// Stored and API name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmee",
            Self::DepositReceived => "acompte_recu",
            Self::Paid => "soldee",
            Self::Cancelled => "annulee",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    /// French label for the history.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmée",
            Self::DepositReceived => "acompte reçu",
            Self::Paid => "soldée",
            Self::Cancelled => "annulée",
        }
    }

    /// Whether the nights of the booking are unavailable.
    #[must_use]
    pub fn blocks_dates(self) -> bool {
        self != Self::Cancelled
    }

    /// Status of a booking that is not cancelled, from what was paid.
    #[must_use]
    pub fn from_payments(paid: u64, deposit: u64, stay_total: u64) -> Self {
        if paid >= stay_total {
            Self::Paid
        } else if paid >= deposit {
            Self::DepositReceived
        } else {
            Self::Confirmed
        }
    }
}

/// How the guest pays: offline while no payment provider is configured (ADR-0020).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentMode {
    Offline,
    Online,
}

impl PaymentMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Online => "online",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "offline" => Some(Self::Offline),
            "online" => Some(Self::Online),
            _ => None,
        }
    }
}

/// The guest's details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Guest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    /// E.164 (`+33612345678`).
    pub phone: String,
    pub street: String,
    pub postcode: String,
    pub town: String,
    pub country: String,
}

/// A detail of the guest that cannot be kept; the text is shown to visitors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GuestError {
    #[error("{0} : champ obligatoire")]
    Missing(&'static str),
    #[error("{0} : {1} caractères au maximum")]
    TooLong(&'static str, usize),
    #[error("adresse e-mail invalide")]
    Email,
    #[error(
        "numéro de téléphone invalide : indiquez-le au format international, par exemple +33612345678"
    )]
    Phone,
}

impl Guest {
    /// The details trimmed, the phone without spaces, or every problem found.
    ///
    /// # Errors
    ///
    /// Each missing, too long or invalid field.
    pub fn validated(self) -> Result<Self, Vec<GuestError>> {
        let guest = Self {
            first_name: self.first_name.trim().to_owned(),
            last_name: self.last_name.trim().to_owned(),
            email: self.email.trim().to_owned(),
            phone: self
                .phone
                .chars()
                .filter(|c| !matches!(c, ' ' | '.' | '-' | '(' | ')'))
                .collect(),
            street: self.street.trim().to_owned(),
            postcode: self.postcode.trim().to_owned(),
            town: self.town.trim().to_owned(),
            country: self.country.trim().to_owned(),
        };
        let mut errors = Vec::new();
        for (label, value, max) in [
            ("prénom", &guest.first_name, 100),
            ("nom", &guest.last_name, 100),
            ("adresse", &guest.street, 200),
            ("code postal", &guest.postcode, 20),
            ("ville", &guest.town, 100),
            ("pays", &guest.country, 100),
        ] {
            if value.is_empty() {
                errors.push(GuestError::Missing(label));
            } else if value.chars().count() > max {
                errors.push(GuestError::TooLong(label, max));
            }
        }
        if !valid_email(&guest.email) {
            errors.push(GuestError::Email);
        }
        if !valid_phone(&guest.phone) {
            errors.push(GuestError::Phone);
        }
        if errors.is_empty() {
            Ok(guest)
        } else {
            Err(errors)
        }
    }
}

/// Whether a guest may choose `minutes` as arrival time with these rules.
#[must_use]
pub fn valid_arrival_time(minutes: u16, rules: &HouseRules) -> bool {
    minutes >= rules.arrival_from && minutes <= LAST_ARRIVAL && minutes.is_multiple_of(SLOT_MINUTES)
}

/// Public reference: `LPG-2027-0042`.
#[must_use]
pub fn reference(year: i32, number: u32) -> String {
    format!("LPG-{year}-{number:04}")
}

/// An option ticked by the guest, with its price for the stay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookedOption {
    pub option: StayOption,
    pub price: u64,
}

/// Rental terms accepted by the guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermsAcceptance {
    pub page: u64,
    /// Published version of the page.
    pub version: u32,
    #[serde(with = "time::serde::rfc3339")]
    pub accepted_at: OffsetDateTime,
}

/// Copy of everything the booking depends on, made when it is created: later changes of
/// prices, rules, promotions or options never alter it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conditions {
    pub nights: u64,
    pub nights_total: u64,
    pub discount: u64,
    pub promotion: Option<Promotion>,
    pub cleaning_fee: u64,
    pub options: Vec<BookedOption>,
    pub options_total: u64,
    pub stay_total: u64,
    pub deposit: u64,
    pub balance: u64,
    /// `None` when everything was due at booking.
    pub balance_due: Option<Date>,
    pub tourist_tax: u64,
    pub rules: HouseRules,
    /// Payment instructions, sanitised HTML.
    pub payment_instructions: Localized,
    pub terms: TermsAcceptance,
}

/// Why an operation on a booking is refused; the text is shown to the user.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BookingRuleError {
    #[error("vous devez accepter les conditions de location")]
    TermsNotAccepted,
    #[error("heure d'arrivée : une demi-heure entre {} et 23:30", clock_time(*.0))]
    ArrivalTime(u16),
    #[error("message : {MAX_MESSAGE} caractères au maximum")]
    MessageTooLong,
    #[error("montant : supérieur à 0 €")]
    AmountZero,
    #[error("mode de paiement inconnu : {0}")]
    UnknownMethod(String),
    #[error("note : {MAX_NOTE} caractères au maximum")]
    NoteTooLong,
    #[error("indiquez le motif de l'annulation ({MAX_NOTE} caractères au maximum)")]
    CancelReason,
    #[error("la réservation est annulée")]
    Cancelled,
    #[error("l'heure d'arrivée ne peut plus être changée en ligne le jour de l'arrivée")]
    TooLateToChange,
}

/// Formats of dates and amounts in messages and screens.
#[must_use]
pub fn date_text(date: Date, lang: Lang) -> String {
    const FR: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    const EN: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let month = usize::from(date.month() as u8 - 1);
    match lang {
        Lang::Fr => format!("{} {} {}", date.day(), FR[month], date.year()),
        Lang::En => format!("{} {} {}", date.day(), EN[month], date.year()),
    }
}

/// Cents → "1 260,50 €" (French) or "€1,260.50" (English).
#[must_use]
pub fn money_text(cents: u64, lang: Lang) -> String {
    let units = (cents / 100).to_string();
    let (group, decimal) = match lang {
        Lang::Fr => ('\u{202f}', ','),
        Lang::En => (',', '.'),
    };
    let mut grouped = String::new();
    for (index, digit) in units.chars().enumerate() {
        if index > 0 && (units.len() - index).is_multiple_of(3) {
            grouped.push(group);
        }
        grouped.push(digit);
    }
    let amount = format!("{grouped}{decimal}{:02}", cents % 100);
    match lang {
        Lang::Fr => format!("{amount} €"),
        Lang::En => format!("€{amount}"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::date;

    use super::*;
    use crate::domain::calendar::BookingHorizon;
    use crate::domain::house_rules::{Deposit, TouristTax, Weekdays};

    fn rules() -> HouseRules {
        HouseRules {
            capacity: 5,
            min_delay_days: 1,
            arrival_weekdays: Weekdays::ALL,
            departure_weekdays: Weekdays::ALL,
            deposit: Deposit::Percentage { percent: 50 },
            balance_days: 30,
            tourist_tax: TouristTax {
                per_adult_night: 0,
                additional_bp: 0,
                exempt_under: 18,
            },
            horizon: BookingHorizon {
                months: 24,
                until: None,
            },
            arrival_from: 960,
            departure_before: 600,
            pets_allowed: false,
            deposit_alert_days: 7,
            terms_page: Some(3),
        }
    }

    fn guest() -> Guest {
        Guest {
            first_name: " Léa ".to_owned(),
            last_name: "Martin".to_owned(),
            email: "lea@example.test".to_owned(),
            phone: "+33 6 12 34 56 78".to_owned(),
            street: "1 rue des Lilas".to_owned(),
            postcode: "75001".to_owned(),
            town: "Paris".to_owned(),
            country: "France".to_owned(),
        }
    }

    #[test]
    fn f007_s08_status_follows_the_payments() {
        assert_eq!(
            BookingStatus::from_payments(0, 26_000, 52_000),
            BookingStatus::Confirmed
        );
        assert_eq!(
            BookingStatus::from_payments(26_000, 26_000, 52_000),
            BookingStatus::DepositReceived
        );
        assert_eq!(
            BookingStatus::from_payments(52_000, 26_000, 52_000),
            BookingStatus::Paid
        );
        assert!(!BookingStatus::Cancelled.blocks_dates());
        assert!(BookingStatus::Confirmed.blocks_dates());
        for status in BookingStatus::ALL {
            assert_eq!(BookingStatus::parse(status.as_str()), Some(status));
        }
    }

    #[test]
    fn guest_details_are_cleaned_and_checked() {
        let guest = guest().validated().unwrap();
        assert_eq!(guest.first_name, "Léa");
        assert_eq!(guest.phone, "+33612345678");
        let broken = Guest {
            first_name: " ".to_owned(),
            email: "lea".to_owned(),
            phone: "0612345678".to_owned(),
            ..self::guest()
        };
        assert_eq!(
            broken.validated().unwrap_err(),
            vec![
                GuestError::Missing("prénom"),
                GuestError::Email,
                GuestError::Phone
            ]
        );
    }

    #[test]
    fn arrival_time_is_a_half_hour_after_the_arrival_from() {
        assert!(valid_arrival_time(960, &rules()));
        assert!(valid_arrival_time(1410, &rules()));
        assert!(!valid_arrival_time(930, &rules()));
        assert!(!valid_arrival_time(1000, &rules()));
        assert!(!valid_arrival_time(1440, &rules()));
    }

    #[test]
    fn f007_s01_reference_format() {
        assert_eq!(reference(2027, 1), "LPG-2027-0001");
        assert_eq!(reference(2027, 12_345), "LPG-2027-12345");
    }

    #[test]
    fn dates_and_amounts_in_each_language() {
        assert_eq!(
            date_text(date!(2027 - 07 - 10), Lang::Fr),
            "10 juillet 2027"
        );
        assert_eq!(date_text(date!(2027 - 07 - 10), Lang::En), "10 July 2027");
        assert_eq!(money_text(26_000, Lang::Fr), "260,00 €");
        assert_eq!(money_text(126_050, Lang::Fr), "1\u{202f}260,50 €");
        assert_eq!(money_text(126_050, Lang::En), "€1,260.50");
        assert_eq!(money_text(5, Lang::En), "€0.05");
    }
}
