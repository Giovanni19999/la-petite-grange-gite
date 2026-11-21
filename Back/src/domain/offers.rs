//! Promotions and options of the direct booking, offered by stay length (F026). Every value
//! comes from the owners (ADR-0010); amounts in cents, percentages in basis points.

use serde::{Deserialize, Serialize};
use time::Date;

use crate::domain::house_rules::euros;
use crate::domain::page::Localized;

const FULL_BP: u64 = 10_000;
const NAME_MAX: usize = 80;
const DESCRIPTION_MAX: usize = 300;

/// Optional minimum and maximum number of nights, both included.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NightBounds {
    #[serde(default)]
    pub min_nights: Option<u16>,
    #[serde(default)]
    pub max_nights: Option<u16>,
}

impl NightBounds {
    #[must_use]
    pub fn contains(self, nights: u64) -> bool {
        self.min_nights.is_none_or(|min| nights >= u64::from(min))
            && self.max_nights.is_none_or(|max| nights <= u64::from(max))
    }

    fn valid(self) -> bool {
        self.min_nights != Some(0)
            && match (self.min_nights, self.max_nights) {
                (Some(min), Some(max)) => min <= max,
                _ => true,
            }
    }

    /// "dès 7 nuits", "jusqu'à 3 nuits", "de 3 à 6 nuits" or "toute durée".
    #[must_use]
    pub fn describe(self) -> String {
        match (self.min_nights, self.max_nights) {
            (Some(min), Some(max)) => format!("de {min} à {max} nuits"),
            (Some(min), None) => format!("dès {min} nuits"),
            (None, Some(max)) => format!("jusqu'à {max} nuits"),
            (None, None) => "toute durée".to_owned(),
        }
    }
}

/// Discount of a promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Discount {
    /// Share of the nights, in basis points.
    Percent { bp: u16 },
    /// Fixed amount for the whole stay.
    PerStay { amount: u64 },
    /// Fixed amount per night.
    PerNight { amount: u64 },
}

impl Discount {
    /// Discount on `nights_total` for `nights` nights, never more than the nights.
    #[must_use]
    pub fn of(self, nights_total: u64, nights: u64) -> u64 {
        let raw = match self {
            Self::Percent { bp } => (nights_total * u64::from(bp) + FULL_BP / 2) / FULL_BP,
            Self::PerStay { amount } => amount,
            Self::PerNight { amount } => amount * nights,
        };
        raw.min(nights_total)
    }

    fn describe(self) -> String {
        match self {
            Self::Percent { bp } if bp % 100 == 0 => format!("−{} %", bp / 100),
            Self::Percent { bp } => format!("−{},{:02} %", bp / 100, bp % 100),
            Self::PerStay { amount } => format!("−{} par séjour", euros(amount)),
            Self::PerNight { amount } => format!("−{} par nuit", euros(amount)),
        }
    }
}

/// A promotion as the owners describe it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Promotion {
    pub name: Localized,
    pub discount: Discount,
    #[serde(flatten)]
    pub bounds: NightBounds,
    /// On the arrival date, both included.
    #[serde(default)]
    pub valid_from: Option<Date>,
    #[serde(default)]
    pub valid_until: Option<Date>,
    #[serde(default = "active")]
    pub active: bool,
}

fn active() -> bool {
    true
}

impl Promotion {
    /// Whether it applies to a stay of `nights` arriving on `arrival`.
    #[must_use]
    pub fn applies(&self, nights: u64, arrival: Date) -> bool {
        self.active
            && self.bounds.contains(nights)
            && self.valid_from.is_none_or(|from| arrival >= from)
            && self.valid_until.is_none_or(|until| arrival <= until)
    }

    /// "Long séjour : −10 %, dès 7 nuits" for the history.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "{} : {}, {}{}",
            self.name.fr.trim(),
            self.discount.describe(),
            self.bounds.describe(),
            if self.active { "" } else { " (inactive)" }
        )
    }

    /// The promotion, or every problem found.
    ///
    /// # Errors
    ///
    /// Each rule broken.
    pub fn validated(self) -> Result<Self, Vec<OfferError>> {
        let mut errors = name_errors(&self.name);
        match self.discount {
            Discount::Percent { bp } if bp == 0 || u64::from(bp) > FULL_BP => {
                errors.push(OfferError::Percent);
            }
            Discount::PerStay { amount: 0 } | Discount::PerNight { amount: 0 } => {
                errors.push(OfferError::Amount);
            }
            _ => {}
        }
        if !self.bounds.valid() {
            errors.push(OfferError::Bounds);
        }
        if let (Some(from), Some(until)) = (self.valid_from, self.valid_until)
            && from > until
        {
            errors.push(OfferError::Dates);
        }
        if errors.is_empty() {
            Ok(Self {
                name: trimmed(&self.name),
                ..self
            })
        } else {
            Err(errors)
        }
    }
}

/// How the price of an option is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionUnit {
    Stay,
    Night,
    Guest,
    GuestNight,
}

impl OptionUnit {
    /// Stored name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stay => "stay",
            Self::Night => "night",
            Self::Guest => "guest",
            Self::GuestNight => "guest_night",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "stay" => Some(Self::Stay),
            "night" => Some(Self::Night),
            "guest" => Some(Self::Guest),
            "guest_night" => Some(Self::GuestNight),
            _ => None,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Stay => "par séjour",
            Self::Night => "par nuit",
            Self::Guest => "par personne",
            Self::GuestNight => "par personne et par nuit",
        }
    }
}

/// An option the visitor may tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StayOption {
    pub name: Localized,
    #[serde(default)]
    pub description: Localized,
    pub unit: OptionUnit,
    /// In cents; 0 for an offered option.
    pub price: u64,
    #[serde(flatten)]
    pub bounds: NightBounds,
    #[serde(default)]
    pub position: u16,
    #[serde(default = "active")]
    pub active: bool,
}

impl StayOption {
    /// Whether it is offered for a stay of `nights`.
    #[must_use]
    pub fn offered(&self, nights: u64) -> bool {
        self.active && self.bounds.contains(nights)
    }

    /// Price for `nights` nights and `guests` guests.
    #[must_use]
    pub fn price_for(&self, nights: u64, guests: u64) -> u64 {
        self.price
            * match self.unit {
                OptionUnit::Stay => 1,
                OptionUnit::Night => nights,
                OptionUnit::Guest => guests,
                OptionUnit::GuestNight => guests * nights,
            }
    }

    /// "Linge de maison : 15 € par personne, dès 3 nuits" for the history.
    #[must_use]
    pub fn describe(&self) -> String {
        let price = if self.price == 0 {
            "offert".to_owned()
        } else {
            format!("{} {}", euros(self.price), self.unit.describe())
        };
        format!(
            "{} : {}, {}{}",
            self.name.fr.trim(),
            price,
            self.bounds.describe(),
            if self.active { "" } else { " (inactive)" }
        )
    }

    /// The option, or every problem found.
    ///
    /// # Errors
    ///
    /// Each rule broken.
    pub fn validated(self) -> Result<Self, Vec<OfferError>> {
        let mut errors = name_errors(&self.name);
        if self.description.fr.chars().count() > DESCRIPTION_MAX
            || self.description.en.chars().count() > DESCRIPTION_MAX
        {
            errors.push(OfferError::Description);
        }
        if !self.bounds.valid() {
            errors.push(OfferError::Bounds);
        }
        if errors.is_empty() {
            Ok(Self {
                name: trimmed(&self.name),
                description: trimmed(&self.description),
                ..self
            })
        } else {
            Err(errors)
        }
    }
}

/// A promotion or option that cannot be saved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OfferError {
    #[error("le nom en français est obligatoire")]
    MissingName,
    #[error("le nom fait au plus {NAME_MAX} caractères")]
    NameTooLong,
    #[error("la description fait au plus {DESCRIPTION_MAX} caractères")]
    Description,
    #[error("la remise en pourcentage est entre 0 et 100 %")]
    Percent,
    #[error("la remise doit être supérieure à 0 €")]
    Amount,
    #[error("le minimum de nuits doit être d'au moins 1 et ne pas dépasser le maximum")]
    Bounds,
    #[error("la date de début doit être avant la date de fin")]
    Dates,
}

fn name_errors(name: &Localized) -> Vec<OfferError> {
    let mut errors = Vec::new();
    if name.fr.trim().is_empty() {
        errors.push(OfferError::MissingName);
    }
    if name.fr.trim().chars().count() > NAME_MAX || name.en.trim().chars().count() > NAME_MAX {
        errors.push(OfferError::NameTooLong);
    }
    errors
}

fn trimmed(text: &Localized) -> Localized {
    Localized {
        fr: text.fr.trim().to_owned(),
        en: text.en.trim().to_owned(),
    }
}

/// The largest discount among the promotions that apply, with the index of its promotion.
#[must_use]
pub fn best_promotion<'a>(
    promotions: impl IntoIterator<Item = &'a Promotion>,
    nights_total: u64,
    nights: u64,
    arrival: Date,
) -> Option<(usize, u64)> {
    promotions
        .into_iter()
        .enumerate()
        .filter(|(_, promotion)| promotion.applies(nights, arrival))
        .map(|(index, promotion)| (index, promotion.discount.of(nights_total, nights)))
        .filter(|(_, discount)| *discount > 0)
        // Ties keep the first promotion.
        .fold(None, |best: Option<(usize, u64)>, candidate| match best {
            Some((_, amount)) if amount >= candidate.1 => best,
            _ => Some(candidate),
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::date;

    use super::*;

    const ARRIVAL: Date = date!(2027 - 07 - 10);

    fn name(fr: &str) -> Localized {
        Localized {
            fr: fr.to_owned(),
            en: String::new(),
        }
    }

    fn promotion(discount: Discount, min: Option<u16>, max: Option<u16>) -> Promotion {
        Promotion {
            name: name("Long séjour"),
            discount,
            bounds: NightBounds {
                min_nights: min,
                max_nights: max,
            },
            valid_from: None,
            valid_until: None,
            active: true,
        }
    }

    fn linen() -> StayOption {
        StayOption {
            name: name("Linge de maison"),
            description: Localized::default(),
            unit: OptionUnit::Guest,
            price: 1_500,
            bounds: NightBounds {
                min_nights: Some(3),
                max_nights: None,
            },
            position: 0,
            active: true,
        }
    }

    #[test]
    fn f026_s01_promotion_from_7_nights() {
        let ten = promotion(Discount::Percent { bp: 1000 }, Some(7), None);
        assert_eq!(best_promotion([&ten], 70_000, 7, ARRIVAL), Some((0, 7_000)));
        assert_eq!(best_promotion([&ten], 60_000, 6, ARRIVAL), None);
        assert_eq!(ten.describe(), "Long séjour : −10 %, dès 7 nuits");
    }

    #[test]
    fn f026_s02_bounds_are_optional_and_inclusive() {
        let short = promotion(Discount::PerNight { amount: 500 }, None, Some(3));
        for nights in 1..=3 {
            assert!(short.applies(nights, ARRIVAL));
        }
        assert!(!short.applies(4, ARRIVAL));
        assert_eq!(short.discount.of(30_000, 3), 1_500);
        assert_eq!(
            Discount::PerStay { amount: 50_000 }.of(30_000, 3),
            30_000,
            "never more than the nights"
        );
    }

    #[test]
    fn f026_s03_best_promotion_only() {
        let ten = promotion(Discount::Percent { bp: 1000 }, Some(7), None);
        let fifteen = promotion(Discount::Percent { bp: 1500 }, Some(14), None);
        assert_eq!(
            best_promotion([&ten, &fifteen], 140_000, 14, ARRIVAL),
            Some((1, 21_000))
        );
    }

    #[test]
    fn f026_validity_dates_and_inactive_promotions() {
        let mut summer = promotion(Discount::Percent { bp: 1000 }, None, None);
        summer.valid_from = Some(date!(2027 - 07 - 01));
        summer.valid_until = Some(date!(2027 - 07 - 10));
        assert!(summer.applies(2, ARRIVAL));
        assert!(!summer.applies(2, date!(2027 - 07 - 11)));
        summer.active = false;
        assert!(!summer.applies(2, ARRIVAL));
    }

    #[test]
    fn f026_s04_option_offered_by_stay_length() {
        let linen = linen();
        assert!(!linen.offered(2));
        assert!(linen.offered(3));
        assert_eq!(linen.price_for(3, 2), 3_000);
        let cot = StayOption {
            unit: OptionUnit::Stay,
            price: 0,
            bounds: NightBounds::default(),
            ..linen.clone()
        };
        assert_eq!(cot.price_for(5, 4), 0);
        let breakfast = StayOption {
            unit: OptionUnit::GuestNight,
            price: 800,
            ..linen.clone()
        };
        assert_eq!(breakfast.price_for(3, 2), 4_800);
        assert_eq!(
            linen.describe(),
            "Linge de maison : 15 € par personne, dès 3 nuits"
        );
    }

    #[test]
    fn f026_offers_are_validated() {
        let broken = Promotion {
            name: name("  "),
            discount: Discount::Percent { bp: 0 },
            bounds: NightBounds {
                min_nights: Some(7),
                max_nights: Some(3),
            },
            valid_from: Some(date!(2027 - 08 - 01)),
            valid_until: Some(date!(2027 - 07 - 01)),
            active: true,
        };
        assert_eq!(
            broken.validated().unwrap_err(),
            vec![
                OfferError::MissingName,
                OfferError::Percent,
                OfferError::Bounds,
                OfferError::Dates
            ]
        );
        let zero = promotion(Discount::PerStay { amount: 0 }, Some(0), None);
        assert_eq!(
            zero.validated().unwrap_err(),
            vec![OfferError::Amount, OfferError::Bounds]
        );
        let parsed: Promotion = serde_json::from_value(serde_json::json!({
            "name": { "fr": " Semaine " },
            "discount": { "kind": "percent", "bp": 1000 },
            "min_nights": 7
        }))
        .unwrap();
        assert_eq!(parsed.validated().unwrap().name.fr, "Semaine");
    }
}
