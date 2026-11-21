//! Prices per date range and per channel (F006, ADR-0008): typed by hand, never computed.
//! The only calculations here are helps (suggestion, typical stays) and the cutting of ranges.
//! Amounts in cents, rates in basis points (1550 = 15.50 %).

use serde::{Deserialize, Serialize};
use time::{Date, Duration};

use crate::domain::calendar::DateRange;

/// Minimum nights of a range or of the cottage.
pub const MIN_NIGHTS: std::ops::RangeInclusive<u8> = 1..=30;
/// Longest label of a range.
pub const MAX_RANGE_LABEL_LEN: usize = 60;
/// Basis points of 100 %.
const FULL: u64 = 10_000;
/// VAT on the commission of the platforms, in basis points (20 %).
const VAT_ON_COMMISSION_BP: u64 = 2_000;
/// Copying a year keeps the weekdays: 52 weeks later.
const WEEKS_IN_YEAR_DAYS: i64 = 52 * 7;

/// What the guest pays (`listed`) and what the owners keep (`net`) per night, in cents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelPrice {
    pub listed: u64,
    pub net: u64,
}

/// A rule broken by a price or a range.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PricingError {
    #[error("le prix affiché doit être supérieur à 0")]
    ZeroListed,
    #[error("le prix touché ne peut pas dépasser le prix affiché")]
    NetAboveListed,
    #[error("la date de fin doit être après la date de début")]
    EndBeforeStart,
    #[error("nombre de nuits minimum invalide (de 1 à 30)")]
    MinNights,
    #[error("nom de plage trop long : {max} caractères au maximum", max = MAX_RANGE_LABEL_LEN)]
    LabelTooLong,
    #[error("taux invalide : entre 0 et 100 %")]
    Rate,
    #[error("canal inconnu : {0}")]
    UnknownChannel(String),
}

impl ChannelPrice {
    /// # Errors
    ///
    /// Zero listed price, or net above listed.
    pub fn validated(self) -> Result<Self, PricingError> {
        if self.listed == 0 {
            return Err(PricingError::ZeroListed);
        }
        if self.net > self.listed {
            return Err(PricingError::NetAboveListed);
        }
        Ok(self)
    }
}

/// Settings of a channel used to help typing (never applied to prices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelRates {
    pub commission_bp: u16,
    pub payment_fees_bp: u16,
    pub vat_on_commission: bool,
}

impl ChannelRates {
    /// # Errors
    ///
    /// A rate above 100 %.
    pub fn validated(self) -> Result<Self, PricingError> {
        if u64::from(self.commission_bp) > FULL || u64::from(self.payment_fees_bp) > FULL {
            return Err(PricingError::Rate);
        }
        Ok(self)
    }

    /// Share of the listed price the channel keeps: `(commission + fees) × 1.2 if VAT`.
    #[must_use]
    pub fn effective_bp(&self) -> u64 {
        let base = u64::from(self.commission_bp) + u64::from(self.payment_fees_bp);
        let total = if self.vat_on_commission {
            base * (FULL + VAT_ON_COMMISSION_BP) / FULL
        } else {
            base
        };
        total.min(FULL)
    }

    /// Listed price suggested for a net price, rounded to the euro (help only).
    #[must_use]
    pub fn suggest_listed(&self, net: u64) -> Option<u64> {
        let keep = FULL
            .checked_sub(self.effective_bp())
            .filter(|keep| *keep > 0)?;
        let listed = net * FULL / keep;
        Some(round_to_euro(listed))
    }

    /// Net price suggested for a listed price, rounded to the euro (help only).
    #[must_use]
    pub fn suggest_net(&self, listed: u64) -> u64 {
        round_to_euro(listed * (FULL - self.effective_bp()) / FULL)
    }

    /// Whether the implicit commission `(listed − net) / listed` differs from the channel's
    /// rate by more than `alert_points` percentage points.
    #[must_use]
    pub fn commission_gap(&self, price: ChannelPrice, alert_points: u8) -> bool {
        if price.listed == 0 {
            return false;
        }
        let implicit = (price.listed - price.net.min(price.listed)) * FULL / price.listed;
        implicit.abs_diff(self.effective_bp()) > u64::from(alert_points) * 100
    }
}

fn round_to_euro(cents: u64) -> u64 {
    (cents + 50) / 100 * 100
}

/// A range of nights with its minimum stay (prices are stored per channel next to it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RangeInput {
    pub start: Date,
    pub end: Date,
    #[serde(default)]
    pub label: Option<String>,
    pub min_nights: u8,
}

impl RangeInput {
    /// Checks the range and cleans its label.
    ///
    /// # Errors
    ///
    /// Every broken rule.
    pub fn validated(self) -> Result<Self, Vec<PricingError>> {
        let label = self
            .label
            .map(|value| {
                ammonia::Builder::empty()
                    .clean(&value)
                    .to_string()
                    .trim()
                    .to_owned()
            })
            .filter(|value| !value.is_empty());
        let mut errors = Vec::new();
        if self.end <= self.start {
            errors.push(PricingError::EndBeforeStart);
        }
        if !MIN_NIGHTS.contains(&self.min_nights) {
            errors.push(PricingError::MinNights);
        }
        if label
            .as_ref()
            .is_some_and(|value| value.chars().count() > MAX_RANGE_LABEL_LEN)
        {
            errors.push(PricingError::LabelTooLong);
        }
        if errors.is_empty() {
            Ok(Self { label, ..self })
        } else {
            Err(errors)
        }
    }

    #[must_use]
    pub fn range(&self) -> DateRange {
        DateRange {
            start: self.start,
            end: Some(self.end),
        }
    }
}

/// A stored range, identified by `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranged<Id> {
    pub id: Id,
    pub range: RangeInput,
}

/// What becomes of an existing range when a new one is applied over it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeCut<Id> {
    pub id: Id,
    /// The parts that remain (0: removed, 1: shortened, 2: split around the new range).
    pub pieces: Vec<(Date, Date)>,
}

/// Cuts the existing ranges so that `new` overwrites them: nothing ever overlaps.
#[must_use]
pub fn cut_ranges<Id: Clone>(existing: &[Ranged<Id>], new: &RangeInput) -> Vec<RangeCut<Id>> {
    existing
        .iter()
        .filter(|other| other.range.start < new.end && new.start < other.range.end)
        .map(|other| {
            let mut pieces = Vec::new();
            if other.range.start < new.start {
                pieces.push((other.range.start, new.start));
            }
            if new.end < other.range.end {
                pieces.push((new.end, other.range.end));
            }
            RangeCut {
                id: other.id.clone(),
                pieces,
            }
        })
        .collect()
}

/// Price of a night: the range covering it if it has a price for the channel, otherwise the
/// channel's default price; `None` when the channel is incomplete on that night.
#[must_use]
pub fn night_price(
    range_price: Option<ChannelPrice>,
    default_price: Option<ChannelPrice>,
) -> Option<ChannelPrice> {
    range_price.or(default_price)
}

/// A range moved 52 weeks later (same weekdays), for "copy to next year".
#[must_use]
pub fn next_year(range: &RangeInput) -> RangeInput {
    let shift = Duration::days(WEEKS_IN_YEAR_DAYS);
    RangeInput {
        start: range.start + shift,
        end: range.end + shift,
        ..range.clone()
    }
}

/// Total of a stay of `nights` on a channel, cleaning included: `(listed, net)`.
/// On platforms the commission also applies to the cleaning: its net part is estimated with
/// the channel's rate.
#[must_use]
pub fn typical_stay(
    nights: u64,
    price: ChannelPrice,
    cleaning_fee: u64,
    rates: ChannelRates,
) -> (u64, u64) {
    let net_cleaning = cleaning_fee * (FULL - rates.effective_bp()) / FULL;
    (
        nights * price.listed + cleaning_fee,
        nights * price.net + net_cleaning,
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::date;

    use super::*;

    const BOOKING: ChannelRates = ChannelRates {
        commission_bp: 1500,
        payment_fees_bp: 140,
        vat_on_commission: true,
    };

    fn range(start: Date, end: Date) -> RangeInput {
        RangeInput {
            start,
            end,
            label: None,
            min_nights: 2,
        }
    }

    #[test]
    fn f006_s05_net_above_listed_is_refused() {
        assert_eq!(
            ChannelPrice {
                listed: 10_000,
                net: 12_000
            }
            .validated(),
            Err(PricingError::NetAboveListed)
        );
        assert_eq!(
            ChannelPrice { listed: 0, net: 0 }.validated(),
            Err(PricingError::ZeroListed)
        );
    }

    #[test]
    fn f006_s04_suggestion_with_the_channel_rate() {
        assert_eq!(BOOKING.effective_bp(), 1968);
        assert_eq!(BOOKING.suggest_listed(11_000), Some(13_700));
        assert_eq!(BOOKING.suggest_net(13_700), 11_000);
        let direct = ChannelRates {
            commission_bp: 0,
            payment_fees_bp: 0,
            vat_on_commission: false,
        };
        assert_eq!(direct.suggest_listed(8_000), Some(8_000));
    }

    #[test]
    fn f006_s04_commission_gap_warning() {
        // 130 € / 109 €: 16.2 % implicit, Booking ≈ 19.7 %: within 5 points.
        assert!(!BOOKING.commission_gap(
            ChannelPrice {
                listed: 13_000,
                net: 10_900
            },
            5
        ));
        // 130 € / 125 €: 3.8 % implicit: far from 19.7 %.
        assert!(BOOKING.commission_gap(
            ChannelPrice {
                listed: 13_000,
                net: 12_500
            },
            5
        ));
    }

    #[test]
    fn f006_s03_cutting_ranges() {
        let existing = vec![
            Ranged {
                id: 1,
                range: range(date!(2027 - 07 - 01), date!(2027 - 07 - 31)),
            },
            Ranged {
                id: 2,
                range: range(date!(2027 - 06 - 01), date!(2027 - 06 - 30)),
            },
        ];
        let cuts = cut_ranges(
            &existing,
            &range(date!(2027 - 07 - 10), date!(2027 - 07 - 20)),
        );
        assert_eq!(
            cuts,
            vec![RangeCut {
                id: 1,
                pieces: vec![
                    (date!(2027 - 07 - 01), date!(2027 - 07 - 10)),
                    (date!(2027 - 07 - 20), date!(2027 - 07 - 31))
                ]
            }]
        );
    }

    #[test]
    fn f006_s03_every_overlap_case() {
        let july = vec![Ranged {
            id: 1,
            range: range(date!(2027 - 07 - 10), date!(2027 - 07 - 20)),
        }];
        let pieces = |start, end| cut_ranges(&july, &range(start, end));
        // Covering: removed.
        assert_eq!(
            pieces(date!(2027 - 07 - 01), date!(2027 - 07 - 31))[0].pieces,
            vec![]
        );
        // Overlapping on the left: shortened at the start.
        assert_eq!(
            pieces(date!(2027 - 07 - 05), date!(2027 - 07 - 12))[0].pieces,
            vec![(date!(2027 - 07 - 12), date!(2027 - 07 - 20))]
        );
        // Overlapping on the right: shortened at the end.
        assert_eq!(
            pieces(date!(2027 - 07 - 15), date!(2027 - 07 - 25))[0].pieces,
            vec![(date!(2027 - 07 - 10), date!(2027 - 07 - 15))]
        );
        // Adjacent: untouched.
        assert_eq!(pieces(date!(2027 - 07 - 20), date!(2027 - 07 - 25)), vec![]);
        assert_eq!(pieces(date!(2027 - 07 - 01), date!(2027 - 07 - 10)), vec![]);
    }

    #[test]
    fn f006_s06_missing_price_falls_back_then_is_incomplete() {
        let range = ChannelPrice {
            listed: 11_000,
            net: 10_800,
        };
        let default = ChannelPrice {
            listed: 10_000,
            net: 10_000,
        };
        assert_eq!(night_price(Some(range), Some(default)), Some(range));
        assert_eq!(night_price(None, Some(default)), Some(default));
        assert_eq!(night_price(None, None), None);
    }

    #[test]
    fn f006_s07_typical_stay_with_cleaning() {
        let site = ChannelRates {
            commission_bp: 0,
            payment_fees_bp: 0,
            vat_on_commission: false,
        };
        let price = ChannelPrice {
            listed: 10_000,
            net: 10_000,
        };
        assert_eq!(typical_stay(7, price, 6_000, site), (76_000, 76_000));
        // On Booking 19.68 % of the cleaning goes to the platform.
        assert_eq!(typical_stay(2, price, 6_000, BOOKING).1, 20_000 + 4_819);
    }

    #[test]
    fn f006_s09_next_year_keeps_weekdays() {
        let summer = range(date!(2027 - 07 - 06), date!(2027 - 08 - 31));
        let next = next_year(&summer);
        assert_eq!(next.start, date!(2028 - 07 - 04));
        assert_eq!(next.end, date!(2028 - 08 - 29));
        assert_eq!(next.start.weekday(), summer.start.weekday());
    }

    #[test]
    fn ranges_are_validated() {
        let errors = RangeInput {
            start: date!(2027 - 07 - 10),
            end: date!(2027 - 07 - 10),
            label: Some("x".repeat(MAX_RANGE_LABEL_LEN + 1)),
            min_nights: 0,
        }
        .validated()
        .unwrap_err();
        assert_eq!(
            errors,
            vec![
                PricingError::EndBeforeStart,
                PricingError::MinNights,
                PricingError::LabelTooLong
            ]
        );
        let clean = RangeInput {
            label: Some(" <i>Été</i> ".into()),
            ..range(date!(2027 - 07 - 06), date!(2027 - 08 - 31))
        }
        .validated()
        .unwrap();
        assert_eq!(clean.label.as_deref(), Some("Été"));
    }
}
