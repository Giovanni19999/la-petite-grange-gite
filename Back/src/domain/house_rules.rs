//! House rules of the cottage (F004): who may book, when, and how the stay is paid.
//!
//! Every value comes from the owners' settings (ADR-0010); only technical bounds, which keep a
//! typing mistake from breaking the quote, are written here.

use serde::{Deserialize, Serialize};
use time::{Date, Weekday};

use crate::domain::calendar::BookingHorizon;

/// Technical bounds of the settings.
const MAX_CAPACITY: u8 = 50;
const MAX_DAYS: u16 = 365;
const MAX_HORIZON_MONTHS: u16 = 60;
const MAX_EXEMPT_AGE: u8 = 25;
const FULL_BP: u32 = 10_000;
const PERCENT: u64 = 100;

/// Days of the week allowed for arrivals or departures, as ISO numbers (Monday = 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weekdays(u8);

impl Weekdays {
    /// Every day of the week.
    pub const ALL: Self = Self(0b111_1111);

    /// From the stored bit set (Monday = 1, Sunday = 64); `None` if empty or out of range.
    #[must_use]
    pub fn from_bits(bits: u8) -> Option<Self> {
        (1..=Self::ALL.0).contains(&bits).then_some(Self(bits))
    }

    #[must_use]
    pub fn bits(self) -> u8 {
        self.0
    }

    #[must_use]
    pub fn contains(self, day: Weekday) -> bool {
        self.0 & (1 << (day.number_from_monday() - 1)) != 0
    }

    /// ISO numbers of the allowed days, Monday first.
    #[must_use]
    pub fn numbers(self) -> Vec<u8> {
        (1..=7).filter(|n| self.0 & (1 << (n - 1)) != 0).collect()
    }

    /// French names, e.g. "samedi" or "vendredi, samedi".
    #[must_use]
    pub fn describe(self) -> String {
        const NAMES: [&str; 7] = [
            "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
        ];
        if self == Self::ALL {
            return "tous les jours".to_owned();
        }
        self.numbers()
            .iter()
            .map(|n| NAMES[usize::from(n - 1)])
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl Serialize for Weekdays {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.numbers().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Weekdays {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let numbers = Vec::<u8>::deserialize(deserializer)?;
        let mut bits = 0u8;
        for n in numbers {
            if !(1..=7).contains(&n) {
                return Err(serde::de::Error::custom("jour de la semaine de 1 à 7"));
            }
            bits |= 1 << (n - 1);
        }
        // An empty set is kept as 0 and refused by `HouseRules::validated`.
        Ok(Self(bits))
    }
}

/// Deposit asked at booking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Deposit {
    /// Percentage of the stay total.
    Percentage { percent: u8 },
    /// Fixed amount in cents, capped at the stay total.
    Fixed { amount: u64 },
}

impl Deposit {
    /// Deposit of a stay total, in cents (a percentage is rounded to the cent).
    #[must_use]
    pub fn of(self, stay_total: u64) -> u64 {
        match self {
            Self::Percentage { percent } => {
                (stay_total * u64::from(percent) + PERCENT / 2) / PERCENT
            }
            Self::Fixed { amount } => amount.min(stay_total),
        }
    }
}

/// Tourist tax paid on arrival (the dated rates and the town-hall summary come with F011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TouristTax {
    /// Per adult and per night, in cents.
    pub per_adult_night: u64,
    /// Additional (departmental) tax, in basis points of the tax.
    pub additional_bp: u16,
    /// Guests younger than this are exempted (the quote counts them as children).
    pub exempt_under: u8,
}

impl TouristTax {
    /// Tax of a stay, rounded to the cent.
    #[must_use]
    pub fn of(self, adults: u8, nights: u64) -> u64 {
        let base = u64::from(adults) * nights * self.per_adult_night;
        let full = u64::from(FULL_BP);
        (base * (full + u64::from(self.additional_bp)) + full / 2) / full
    }
}

/// The house rules of F004.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseRules {
    /// Most guests, adults and children counted alike.
    pub capacity: u8,
    /// Days between today and the earliest arrival (1: tomorrow).
    pub min_delay_days: u16,
    pub arrival_weekdays: Weekdays,
    pub departure_weekdays: Weekdays,
    pub deposit: Deposit,
    /// The balance is due this many days before arrival.
    pub balance_days: u16,
    pub tourist_tax: TouristTax,
    /// Booking horizon of F005, edited on the same screen.
    pub horizon: BookingHorizon,
}

/// A house rule that cannot be saved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleError {
    #[error("capacité : de 1 à {MAX_CAPACITY} personnes")]
    Capacity,
    #[error("délai minimum : de 0 à {MAX_DAYS} jours")]
    MinDelay,
    #[error("choisissez au moins un jour d'arrivée")]
    ArrivalWeekdays,
    #[error("choisissez au moins un jour de départ")]
    DepartureWeekdays,
    #[error("acompte : de 1 à 100 %")]
    DepositPercent,
    #[error("acompte : un montant supérieur à 0 €")]
    DepositAmount,
    #[error("solde : de 0 à {MAX_DAYS} jours avant l'arrivée")]
    BalanceDays,
    #[error("taxe additionnelle : de 0 à 100 %")]
    AdditionalTax,
    #[error("âge d'exonération : de 0 à {MAX_EXEMPT_AGE} ans")]
    ExemptAge,
    #[error("réservable : de 1 à {MAX_HORIZON_MONTHS} mois")]
    HorizonMonths,
}

impl HouseRules {
    /// The rules, or every problem found.
    ///
    /// # Errors
    ///
    /// Each rule out of its bounds.
    pub fn validated(self) -> Result<Self, Vec<RuleError>> {
        let mut errors = Vec::new();
        if !(1..=MAX_CAPACITY).contains(&self.capacity) {
            errors.push(RuleError::Capacity);
        }
        if self.min_delay_days > MAX_DAYS {
            errors.push(RuleError::MinDelay);
        }
        if Weekdays::from_bits(self.arrival_weekdays.bits()).is_none() {
            errors.push(RuleError::ArrivalWeekdays);
        }
        if Weekdays::from_bits(self.departure_weekdays.bits()).is_none() {
            errors.push(RuleError::DepartureWeekdays);
        }
        match self.deposit {
            Deposit::Percentage { percent } if !(1..=100).contains(&percent) => {
                errors.push(RuleError::DepositPercent);
            }
            Deposit::Fixed { amount: 0 } => errors.push(RuleError::DepositAmount),
            _ => {}
        }
        if self.balance_days > MAX_DAYS {
            errors.push(RuleError::BalanceDays);
        }
        if u32::from(self.tourist_tax.additional_bp) > FULL_BP {
            errors.push(RuleError::AdditionalTax);
        }
        if self.tourist_tax.exempt_under > MAX_EXEMPT_AGE {
            errors.push(RuleError::ExemptAge);
        }
        if !(1..=MAX_HORIZON_MONTHS).contains(&self.horizon.months) {
            errors.push(RuleError::HorizonMonths);
        }
        if errors.is_empty() {
            Ok(self)
        } else {
            Err(errors)
        }
    }

    /// French summary of what changed, for the history ("Capacité : 5 → 6").
    #[must_use]
    pub fn describe_changes(&self, after: &Self) -> String {
        let mut changes = Vec::new();
        let mut diff = |label: &str, before: String, now: String| {
            if before != now {
                changes.push(format!("{label} : {before} → {now}"));
            }
        };
        diff(
            "Capacité",
            self.capacity.to_string(),
            after.capacity.to_string(),
        );
        diff(
            "Délai minimum",
            days(self.min_delay_days),
            days(after.min_delay_days),
        );
        diff(
            "Arrivées",
            self.arrival_weekdays.describe(),
            after.arrival_weekdays.describe(),
        );
        diff(
            "Départs",
            self.departure_weekdays.describe(),
            after.departure_weekdays.describe(),
        );
        diff("Acompte", deposit(self.deposit), deposit(after.deposit));
        diff(
            "Solde",
            format!("{} avant l'arrivée", days(self.balance_days)),
            format!("{} avant l'arrivée", days(after.balance_days)),
        );
        diff(
            "Taxe de séjour",
            euros(self.tourist_tax.per_adult_night),
            euros(after.tourist_tax.per_adult_night),
        );
        diff(
            "Taxe additionnelle",
            percent_bp(self.tourist_tax.additional_bp),
            percent_bp(after.tourist_tax.additional_bp),
        );
        diff(
            "Exonération",
            format!("moins de {} ans", self.tourist_tax.exempt_under),
            format!("moins de {} ans", after.tourist_tax.exempt_under),
        );
        diff("Réservable", horizon(self.horizon), horizon(after.horizon));
        if changes.is_empty() {
            "Règles du gîte enregistrées sans changement".to_owned()
        } else {
            changes.join(" ; ")
        }
    }
}

fn days(count: u16) -> String {
    if count > 1 {
        format!("{count} jours")
    } else {
        format!("{count} jour")
    }
}

/// Cents → "1,70 €" or "60 €".
#[must_use]
pub fn euros(cents: u64) -> String {
    if cents.is_multiple_of(100) {
        format!("{} €", cents / 100)
    } else {
        format!("{},{:02} €", cents / 100, cents % 100)
    }
}

fn percent_bp(bp: u16) -> String {
    if bp.is_multiple_of(100) {
        format!("{} %", bp / 100)
    } else {
        format!(
            "{},{}",
            bp / 100,
            format!("{:02}", bp % 100).trim_end_matches('0')
        ) + " %"
    }
}

fn deposit(deposit: Deposit) -> String {
    match deposit {
        Deposit::Percentage { percent } => format!("{percent} %"),
        Deposit::Fixed { amount } => euros(amount),
    }
}

fn horizon(horizon: BookingHorizon) -> String {
    match horizon.until {
        Some(until) => format!(
            "{} mois, jusqu'au {:02}/{:02}/{} au plus tard",
            horizon.months,
            until.day(),
            until.month() as u8,
            until.year()
        ),
        None => format!("{} mois", horizon.months),
    }
}

/// First day an arrival is allowed, seen from `today`.
#[must_use]
pub fn earliest_arrival(today: Date, min_delay_days: u16) -> Date {
    today + time::Duration::days(i64::from(min_delay_days))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::date;

    use super::*;

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
        }
    }

    #[test]
    fn f004_s03_deposit_and_tourist_tax() {
        assert_eq!(Deposit::Percentage { percent: 50 }.of(36_000), 18_000);
        assert_eq!(Deposit::Percentage { percent: 30 }.of(33_333), 10_000);
        assert_eq!(Deposit::Fixed { amount: 50_000 }.of(36_000), 36_000);
        let tax = TouristTax {
            per_adult_night: 170,
            additional_bp: 1000,
            exempt_under: 18,
        };
        assert_eq!(tax.of(2, 3), 1122);
    }

    #[test]
    fn f004_s10_weekdays() {
        let saturday = Weekdays::from_bits(1 << 5).unwrap();
        assert!(saturday.contains(Weekday::Saturday));
        assert!(!saturday.contains(Weekday::Wednesday));
        assert_eq!(saturday.numbers(), vec![6]);
        assert_eq!(saturday.describe(), "samedi");
        assert_eq!(Weekdays::ALL.describe(), "tous les jours");
        assert_eq!(Weekdays::from_bits(0), None);
        let parsed: Weekdays = serde_json::from_str("[5, 6]").unwrap();
        assert_eq!(parsed.describe(), "vendredi, samedi");
        assert!(serde_json::from_str::<Weekdays>("[8]").is_err());
    }

    #[test]
    fn f004_s09_rules_are_validated() {
        assert_eq!(rules().validated(), Ok(rules()));
        let broken = HouseRules {
            capacity: 0,
            arrival_weekdays: serde_json::from_str("[]").unwrap(),
            deposit: Deposit::Percentage { percent: 0 },
            tourist_tax: TouristTax {
                additional_bp: 12_000,
                ..rules().tourist_tax
            },
            horizon: BookingHorizon {
                months: 0,
                until: None,
            },
            ..rules()
        };
        assert_eq!(
            broken.validated().unwrap_err(),
            vec![
                RuleError::Capacity,
                RuleError::ArrivalWeekdays,
                RuleError::DepositPercent,
                RuleError::AdditionalTax,
                RuleError::HorizonMonths,
            ]
        );
        let fixed_zero = HouseRules {
            deposit: Deposit::Fixed { amount: 0 },
            ..rules()
        };
        assert_eq!(
            fixed_zero.validated().unwrap_err(),
            vec![RuleError::DepositAmount]
        );
    }

    #[test]
    fn f004_s09_changes_are_described() {
        let after = HouseRules {
            capacity: 6,
            tourist_tax: TouristTax {
                per_adult_night: 170,
                additional_bp: 1000,
                exempt_under: 18,
            },
            horizon: BookingHorizon {
                months: 24,
                until: Some(date!(2028 - 06 - 30)),
            },
            ..rules()
        };
        assert_eq!(
            rules().describe_changes(&after),
            "Capacité : 5 → 6 ; Taxe de séjour : 0 € → 1,70 € ; Taxe additionnelle : 0 % → 10 % ; \
             Réservable : 24 mois → 24 mois, jusqu'au 30/06/2028 au plus tard"
        );
        assert_eq!(
            rules().describe_changes(&rules()),
            "Règles du gîte enregistrées sans changement"
        );
    }

    #[test]
    fn earliest_arrival_adds_the_delay() {
        assert_eq!(
            earliest_arrival(date!(2026 - 11 - 21), 1),
            date!(2026 - 11 - 22)
        );
        assert_eq!(
            earliest_arrival(date!(2026 - 11 - 21), 0),
            date!(2026 - 11 - 21)
        );
    }
}
