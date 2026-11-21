//! Calendar rules of a cottage (F005): date ranges, blocked periods, booking horizon and the
//! nights visitors cannot book. Pure functions: no I/O, no clock (the caller gives "today").

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset, Weekday};

/// Longest label of a family or private-event period.
pub const MAX_LABEL_LEN: usize = 80;
/// Longest reason of a period.
pub const MAX_REASON_LEN: usize = 300;
/// Number of people of a family or private-event period.
pub const PEOPLE: std::ops::RangeInclusive<u8> = 1..=99;

/// Nights from `start` (first night) to `end` (excluded, the day after the last night), or
/// open-ended when `end` is `None` ("closed until further notice").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Date,
    pub end: Option<Date>,
}

impl DateRange {
    /// Whether the night of `date` (from `date` to the next day) is inside the range.
    #[must_use]
    pub fn contains_night(&self, date: Date) -> bool {
        date >= self.start && self.end.is_none_or(|end| date < end)
    }

    /// Whether the two ranges share at least one night.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        let starts_before_other_ends = other.end.is_none_or(|end| self.start < end);
        let other_starts_before_end = self.end.is_none_or(|end| other.start < end);
        starts_before_other_ends && other_starts_before_end
    }

    /// Number of nights, `None` when open-ended.
    #[must_use]
    pub fn nights(&self) -> Option<i64> {
        self.end.map(|end| (end - self.start).whole_days())
    }
}

/// Why a period is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedKind {
    Closure,
    Family,
    PrivateEvent,
}

impl BlockedKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closure => "closure",
            Self::Family => "family",
            Self::PrivateEvent => "private_event",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "closure" => Some(Self::Closure),
            "family" => Some(Self::Family),
            "private_event" => Some(Self::PrivateEvent),
            _ => None,
        }
    }

    /// Family and private events are private occupancy: they need a label.
    #[must_use]
    pub fn is_private(self) -> bool {
        !matches!(self, Self::Closure)
    }
}

/// A blocked period as the owners describe it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockedPeriodInput {
    pub start: Date,
    /// `None`: closed until further notice.
    #[serde(default)]
    pub end: Option<Date>,
    pub kind: BlockedKind,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub people: Option<u8>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// A rule broken by a blocked period.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PeriodError {
    #[error("la date de fin doit être après la date de début")]
    EndBeforeStart,
    #[error("un libellé est obligatoire pour une période famille ou un événement privé")]
    MissingLabel,
    #[error("libellé trop long : {max} caractères au maximum", max = MAX_LABEL_LEN)]
    LabelTooLong,
    #[error("motif trop long : {max} caractères au maximum", max = MAX_REASON_LEN)]
    ReasonTooLong,
    #[error("nombre de personnes invalide (de 1 à 99)")]
    People,
}

impl BlockedPeriodInput {
    /// Checks the period and cleans its texts (plain text, trimmed, empty → none).
    ///
    /// # Errors
    ///
    /// Every broken rule.
    pub fn validated(self) -> Result<Self, Vec<PeriodError>> {
        let clean = |text: Option<String>| {
            text.map(|value| {
                ammonia::Builder::empty()
                    .clean(&value)
                    .to_string()
                    .trim()
                    .to_owned()
            })
            .filter(|value| !value.is_empty())
        };
        let label = clean(self.label);
        let reason = clean(self.reason);
        let mut errors = Vec::new();
        if self.end.is_some_and(|end| end <= self.start) {
            errors.push(PeriodError::EndBeforeStart);
        }
        if self.kind.is_private() && label.is_none() {
            errors.push(PeriodError::MissingLabel);
        }
        if label
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_LABEL_LEN)
        {
            errors.push(PeriodError::LabelTooLong);
        }
        if reason
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_REASON_LEN)
        {
            errors.push(PeriodError::ReasonTooLong);
        }
        if self.people.is_some_and(|people| !PEOPLE.contains(&people)) {
            errors.push(PeriodError::People);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Self {
            label,
            reason,
            // A plain closure has no guests.
            people: if self.kind.is_private() {
                self.people
            } else {
                None
            },
            ..self
        })
    }

    #[must_use]
    pub fn range(&self) -> DateRange {
        DateRange {
            start: self.start,
            end: self.end,
        }
    }
}

/// How far ahead visitors may book: a rolling number of months, and an optional fixed last
/// date; the nearest applies (house rules, ADR-0010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BookingHorizon {
    pub months: u16,
    #[serde(default)]
    pub until: Option<Date>,
}

impl BookingHorizon {
    /// First night that can no longer be booked, seen from `today`.
    #[must_use]
    pub fn first_closed_night(&self, today: Date) -> Date {
        let rolling = add_months(today, self.months);
        self.until.map_or(rolling, |until| until.min(rolling))
    }
}

/// `date` plus `months`, clamped to the last day of the target month (31 Jan + 1 = 28/29 Feb).
#[must_use]
pub fn add_months(date: Date, months: u16) -> Date {
    let total = i32::from(date.month() as u8) - 1 + i32::from(months);
    let year = date.year() + total.div_euclid(12);
    let month = Month::try_from(u8::try_from(total.rem_euclid(12) + 1).unwrap_or(1))
        .unwrap_or(Month::January);
    let day = date.day().min(time::util::days_in_month(month, year));
    Date::from_calendar_date(year, month, day).unwrap_or(date)
}

/// A night that cannot be booked, and nothing else: visitors never learn why.
pub type UnavailableNights = Vec<Date>;

/// Nights from `from` (included) to `to` (excluded) that visitors cannot book: covered by a
/// blocked period or a stay, or beyond the booking horizon seen from `today`.
#[must_use]
pub fn unavailable_nights(
    from: Date,
    to: Date,
    blocked: &[DateRange],
    stays: &[DateRange],
    horizon: BookingHorizon,
    today: Date,
) -> UnavailableNights {
    let closed_from = horizon.first_closed_night(today);
    let mut nights = BTreeSet::new();
    let mut night = from;
    while night < to {
        let taken = night >= closed_from
            || blocked.iter().any(|range| range.contains_night(night))
            || stays.iter().any(|range| range.contains_night(night));
        if taken {
            nights.insert(night);
        }
        match night.next_day() {
            Some(next) => night = next,
            None => break,
        }
    }
    nights.into_iter().collect()
}

/// Calendar date in Europe/Paris of a UTC instant (CET, CEST from the last Sunday of March
/// 01:00 UTC to the last Sunday of October 01:00 UTC, as in every EU country).
#[must_use]
pub fn paris_date(instant: OffsetDateTime) -> Date {
    let utc = instant.to_offset(UtcOffset::UTC);
    let year = utc.year();
    let summer_starts = last_sunday_at_one_utc(year, Month::March);
    let summer_ends = last_sunday_at_one_utc(year, Month::October);
    let offset_hours = if utc >= summer_starts && utc < summer_ends {
        2
    } else {
        1
    };
    (utc + Duration::hours(offset_hours)).date()
}

fn last_sunday_at_one_utc(year: i32, month: Month) -> OffsetDateTime {
    let last_day = time::util::days_in_month(month, year);
    let mut date = Date::from_calendar_date(year, month, last_day).unwrap_or(Date::MIN);
    while date.weekday() != Weekday::Sunday {
        date = date.previous_day().unwrap_or(date);
    }
    PrimitiveDateTime::new(date, Time::from_hms(1, 0, 0).unwrap_or(Time::MIDNIGHT)).assume_utc()
}

impl fmt::Display for BlockedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::{date, datetime};

    use super::*;

    fn range(start: Date, end: Option<Date>) -> DateRange {
        DateRange { start, end }
    }

    const HORIZON: BookingHorizon = BookingHorizon {
        months: 24,
        until: None,
    };

    #[test]
    fn f005_s01_end_is_excluded() {
        let closure = range(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15)));
        assert!(closure.contains_night(date!(2027 - 03 - 10)));
        assert!(closure.contains_night(date!(2027 - 03 - 14)));
        assert!(!closure.contains_night(date!(2027 - 03 - 15)));
        assert!(!closure.contains_night(date!(2027 - 03 - 09)));
        assert_eq!(closure.nights(), Some(5));
    }

    #[test]
    fn f005_s01_unavailable_nights_of_a_closure() {
        let nights = unavailable_nights(
            date!(2027 - 03 - 08),
            date!(2027 - 03 - 17),
            &[range(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15)))],
            &[],
            HORIZON,
            date!(2026 - 11 - 21),
        );
        assert_eq!(
            nights,
            vec![
                date!(2027 - 03 - 10),
                date!(2027 - 03 - 11),
                date!(2027 - 03 - 12),
                date!(2027 - 03 - 13),
                date!(2027 - 03 - 14)
            ]
        );
    }

    #[test]
    fn f005_s03_label_required_for_private_periods() {
        let input = BlockedPeriodInput {
            start: date!(2027 - 08 - 01),
            end: Some(date!(2027 - 08 - 08)),
            kind: BlockedKind::PrivateEvent,
            label: Some("  ".into()),
            people: None,
            reason: None,
        };
        assert_eq!(input.validated(), Err(vec![PeriodError::MissingLabel]));
    }

    #[test]
    fn f005_s02_family_period_is_cleaned() {
        let input = BlockedPeriodInput {
            start: date!(2027 - 08 - 01),
            end: Some(date!(2027 - 08 - 08)),
            kind: BlockedKind::Family,
            label: Some(" <b>Mamie</b> et Papi ".into()),
            people: Some(4),
            reason: Some(String::new()),
        }
        .validated()
        .unwrap();
        assert_eq!(input.label.as_deref(), Some("Mamie et Papi"));
        assert_eq!(input.reason, None);
        assert_eq!(input.people, Some(4));
    }

    #[test]
    fn invalid_periods_report_every_rule() {
        let errors = BlockedPeriodInput {
            start: date!(2027 - 08 - 08),
            end: Some(date!(2027 - 08 - 08)),
            kind: BlockedKind::Family,
            label: Some("x".repeat(MAX_LABEL_LEN + 1)),
            people: Some(0),
            reason: Some("y".repeat(MAX_REASON_LEN + 1)),
        }
        .validated()
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                PeriodError::EndBeforeStart,
                PeriodError::LabelTooLong,
                PeriodError::ReasonTooLong,
                PeriodError::People
            ]
        );
    }

    #[test]
    fn closures_have_no_people() {
        let input = BlockedPeriodInput {
            start: date!(2027 - 03 - 10),
            end: Some(date!(2027 - 03 - 15)),
            kind: BlockedKind::Closure,
            label: None,
            people: Some(3),
            reason: Some("Travaux".into()),
        }
        .validated()
        .unwrap();
        assert_eq!(input.people, None);
    }

    #[test]
    fn f005_s05_open_ended_closure_never_ends() {
        let closed = range(date!(2026 - 11 - 01), None);
        assert!(closed.contains_night(date!(2040 - 01 - 01)));
        assert!(!closed.contains_night(date!(2026 - 10 - 31)));
        assert_eq!(closed.nights(), None);
    }

    #[test]
    fn f005_s06_overlaps() {
        let march = range(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15)));
        assert!(march.overlaps(&range(date!(2027 - 03 - 14), Some(date!(2027 - 03 - 20)))));
        assert!(!march.overlaps(&range(date!(2027 - 03 - 15), Some(date!(2027 - 03 - 20)))));
        assert!(!march.overlaps(&range(date!(2027 - 03 - 01), Some(date!(2027 - 03 - 10)))));
        assert!(march.overlaps(&range(date!(2027 - 03 - 01), None)));
        assert!(range(date!(2027 - 01 - 01), None).overlaps(&range(date!(2030 - 01 - 01), None)));
    }

    #[test]
    fn f005_s04_stays_are_unavailable() {
        let nights = unavailable_nights(
            date!(2027 - 06 - 04),
            date!(2027 - 06 - 10),
            &[],
            &[range(date!(2027 - 06 - 05), Some(date!(2027 - 06 - 09)))],
            HORIZON,
            date!(2026 - 11 - 21),
        );
        assert_eq!(nights.first(), Some(&date!(2027 - 06 - 05)));
        assert_eq!(nights.last(), Some(&date!(2027 - 06 - 08)));
        assert_eq!(nights.len(), 4);
    }

    #[test]
    fn f005_s08_horizon_closes_far_dates() {
        let today = date!(2026 - 11 - 21);
        assert_eq!(HORIZON.first_closed_night(today), date!(2028 - 11 - 21));
        let nights = unavailable_nights(
            date!(2028 - 11 - 19),
            date!(2028 - 11 - 23),
            &[],
            &[],
            HORIZON,
            today,
        );
        assert_eq!(nights, vec![date!(2028 - 11 - 21), date!(2028 - 11 - 22)]);

        let fixed = BookingHorizon {
            months: 24,
            until: Some(date!(2027 - 06 - 30)),
        };
        assert_eq!(fixed.first_closed_night(today), date!(2027 - 06 - 30));
    }

    #[test]
    fn months_are_added_to_the_end_of_short_months() {
        assert_eq!(add_months(date!(2027 - 01 - 31), 1), date!(2027 - 02 - 28));
        assert_eq!(add_months(date!(2027 - 11 - 15), 3), date!(2028 - 02 - 15));
        assert_eq!(add_months(date!(2026 - 02 - 28), 24), date!(2028 - 02 - 28));
    }

    #[test]
    fn paris_date_follows_summer_time() {
        // Winter (UTC+1): 23:30 UTC is already the next day in Paris.
        assert_eq!(
            paris_date(datetime!(2026-12-31 23:30 UTC)),
            date!(2027 - 01 - 01)
        );
        // Summer (UTC+2): 22:30 UTC is the next day in Paris.
        assert_eq!(
            paris_date(datetime!(2027-07-14 22:30 UTC)),
            date!(2027 - 07 - 15)
        );
        assert_eq!(
            paris_date(datetime!(2027-07-14 21:30 UTC)),
            date!(2027 - 07 - 14)
        );
        // Change on the last Sunday of March 2027 (28 March) at 01:00 UTC.
        assert_eq!(
            paris_date(datetime!(2027-03-28 00:59 UTC)),
            date!(2027 - 03 - 28)
        );
        assert_eq!(
            paris_date(datetime!(2027-03-27 22:30 UTC)),
            date!(2027 - 03 - 27)
        );
        assert_eq!(
            paris_date(datetime!(2027-03-28 22:30 UTC)),
            date!(2027 - 03 - 29)
        );
    }
}
