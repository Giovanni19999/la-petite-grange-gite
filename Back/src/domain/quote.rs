//! Quote of a stay on the site (F004): amounts in cents, computed only here, never in the
//! browser.

use time::Date;

use crate::domain::house_rules::{HouseRules, Weekdays, earliest_arrival};
use crate::domain::offers::{Promotion, StayOption, best_promotion};

/// What the visitor asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StayRequest {
    pub arrival: Date,
    /// Day of departure (the last night is the day before).
    pub departure: Date,
    pub adults: u8,
    /// Guests under the exemption age of the tourist tax.
    pub children: u8,
    /// Ids of the ticked options (F026).
    pub options: Vec<u64>,
}

/// An option offered for the stay, with its price for this stay (F026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuotedOption {
    pub id: u64,
    pub price: u64,
    pub selected: bool,
}

/// One night as visitors may book it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NightOffer {
    pub date: Date,
    /// Site listed price; `None` when the night cannot be booked (taken, closed, no price).
    pub price: Option<u64>,
    /// Minimum stay when arriving this night.
    pub min_nights: u8,
}

/// An exact quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    pub nights: u64,
    pub nights_total: u64,
    /// Discount of the best promotion that applies (F026), on the nights only.
    pub discount: u64,
    /// Index of that promotion in the context.
    pub promotion: Option<usize>,
    pub cleaning_fee: u64,
    /// Options offered for this stay, in display order.
    pub options: Vec<QuotedOption>,
    /// Total of the ticked options.
    pub options_total: u64,
    /// Nights − discount + cleaning + ticked options: what the deposit and the balance share.
    pub stay_total: u64,
    /// Due at booking.
    pub deposit: u64,
    pub balance: u64,
    /// `None` when everything is due at booking (the balance date is already reached).
    pub balance_due: Option<Date>,
    /// Paid on arrival, apart from the stay total.
    pub tourist_tax: u64,
}

/// Why a quote is refused; the text is shown to visitors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum QuoteRefusal {
    #[error("la date de départ doit être après la date d'arrivée")]
    DatesOrder,
    #[error("au moins un adulte est nécessaire")]
    NoAdult,
    #[error("{0} personnes au maximum")]
    Capacity(u8),
    #[error("arrivée au plus tôt le {}", day(*.0))]
    TooSoon(Date),
    #[error("arrivée possible : {}", .0.describe())]
    ArrivalDay(Weekdays),
    #[error("départ possible : {}", .0.describe())]
    DepartureDay(Weekdays),
    #[error("dates indisponibles")]
    Unavailable,
    #[error("{0} nuits minimum pour cette date d'arrivée")]
    MinimumStay(u8),
    #[error("une option choisie n'est pas proposée pour ce séjour")]
    OptionNotOffered,
}

fn day(date: Date) -> String {
    format!(
        "{:02}/{:02}/{}",
        date.day(),
        date.month() as u8,
        date.year()
    )
}

/// Everything the quote depends on besides the request.
#[derive(Debug, Clone, Copy)]
pub struct QuoteContext<'a> {
    pub today: Date,
    pub rules: &'a HouseRules,
    /// Offers of the nights of the stay (a missing night counts as unavailable).
    pub nights: &'a [NightOffer],
    pub cleaning_fee: u64,
    /// Promotions of the site (F026).
    pub promotions: &'a [Promotion],
    /// Options of the site with their id, in display order (F026).
    pub options: &'a [(u64, StayOption)],
}

/// Guests, minimum delay and weekdays of the house rules.
fn house_rule_refusals(
    request: &StayRequest,
    rules: &HouseRules,
    today: Date,
) -> Vec<QuoteRefusal> {
    let mut refusals = Vec::new();
    if request.adults == 0 {
        refusals.push(QuoteRefusal::NoAdult);
    }
    if u16::from(request.adults) + u16::from(request.children) > u16::from(rules.capacity) {
        refusals.push(QuoteRefusal::Capacity(rules.capacity));
    }
    let earliest = earliest_arrival(today, rules.min_delay_days);
    if request.arrival < earliest {
        refusals.push(QuoteRefusal::TooSoon(earliest));
    }
    if !rules.arrival_weekdays.contains(request.arrival.weekday()) {
        refusals.push(QuoteRefusal::ArrivalDay(rules.arrival_weekdays));
    }
    if !rules
        .departure_weekdays
        .contains(request.departure.weekday())
    {
        refusals.push(QuoteRefusal::DepartureDay(rules.departure_weekdays));
    }
    refusals
}

/// The quote of `request`, or every reason to refuse it.
///
/// # Errors
///
/// The reasons the stay cannot be booked, in a stable order.
pub fn quote(request: &StayRequest, context: QuoteContext<'_>) -> Result<Quote, Vec<QuoteRefusal>> {
    if request.departure <= request.arrival {
        return Err(vec![QuoteRefusal::DatesOrder]);
    }
    let rules = context.rules;
    let mut refusals = house_rule_refusals(request, rules, context.today);

    let mut prices = Vec::new();
    let mut night = request.arrival;
    while night < request.departure {
        prices.push(
            context
                .nights
                .iter()
                .find(|offer| offer.date == night)
                .and_then(|offer| offer.price),
        );
        match night.next_day() {
            Some(next) => night = next,
            None => break,
        }
    }
    if prices.iter().any(Option::is_none) {
        refusals.push(QuoteRefusal::Unavailable);
    }
    let nights = prices.len() as u64;
    let min_nights = context
        .nights
        .iter()
        .find(|offer| offer.date == request.arrival)
        .map_or(1, |offer| offer.min_nights);
    if nights < u64::from(min_nights) {
        refusals.push(QuoteRefusal::MinimumStay(min_nights));
    }
    let offered: Vec<&(u64, StayOption)> = context
        .options
        .iter()
        .filter(|(_, option)| option.offered(nights))
        .collect();
    if request
        .options
        .iter()
        .any(|id| !offered.iter().any(|(offered_id, _)| offered_id == id))
    {
        refusals.push(QuoteRefusal::OptionNotOffered);
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }

    let nights_total: u64 = prices.into_iter().flatten().sum();
    let best = best_promotion(context.promotions, nights_total, nights, request.arrival);
    let discount = best.map_or(0, |(_, amount)| amount);
    let guests = u64::from(request.adults) + u64::from(request.children);
    let options: Vec<QuotedOption> = offered
        .iter()
        .map(|(id, option)| QuotedOption {
            id: *id,
            price: option.price_for(nights, guests),
            selected: request.options.contains(id),
        })
        .collect();
    let options_total = options
        .iter()
        .filter(|option| option.selected)
        .map(|option| option.price)
        .sum();
    let stay_total = nights_total - discount + context.cleaning_fee + options_total;
    let balance_date = request.arrival - time::Duration::days(i64::from(rules.balance_days));
    let (deposit, balance_due) = if balance_date <= context.today {
        (stay_total, None)
    } else {
        (rules.deposit.of(stay_total), Some(balance_date))
    };
    Ok(Quote {
        nights,
        nights_total,
        discount,
        promotion: best.map(|(index, _)| index),
        cleaning_fee: context.cleaning_fee,
        options,
        options_total,
        stay_total,
        deposit,
        balance: stay_total - deposit,
        balance_due,
        tourist_tax: rules.tourist_tax.of(request.adults, nights),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use time::macros::date;

    use super::*;
    use crate::domain::calendar::BookingHorizon;
    use crate::domain::house_rules::{Deposit, TouristTax};
    use crate::domain::offers::{Discount, NightBounds, OptionUnit};
    use crate::domain::page::Localized;

    const TODAY: Date = date!(2027 - 03 - 01);

    fn rules() -> HouseRules {
        HouseRules {
            capacity: 5,
            min_delay_days: 1,
            arrival_weekdays: Weekdays::ALL,
            departure_weekdays: Weekdays::ALL,
            deposit: Deposit::Percentage { percent: 50 },
            balance_days: 30,
            tourist_tax: TouristTax {
                per_adult_night: 170,
                additional_bp: 1000,
                exempt_under: 18,
            },
            horizon: BookingHorizon {
                months: 24,
                until: None,
            },
        }
    }

    /// Free nights at 100 € from 1 March to 31 May 2027, minimum 2 nights.
    fn offers() -> Vec<NightOffer> {
        let mut offers = Vec::new();
        let mut night = TODAY;
        while night < date!(2027 - 06 - 01) {
            offers.push(NightOffer {
                date: night,
                price: Some(10_000),
                min_nights: 2,
            });
            night = night.next_day().unwrap();
        }
        offers
    }

    fn ask(
        arrival: Date,
        departure: Date,
        adults: u8,
        children: u8,
        rules: &HouseRules,
        nights: &[NightOffer],
    ) -> Result<Quote, Vec<QuoteRefusal>> {
        quote(
            &StayRequest {
                arrival,
                departure,
                adults,
                children,
                options: Vec::new(),
            },
            QuoteContext {
                today: TODAY,
                rules,
                nights,
                cleaning_fee: 6_000,
                promotions: &[],
                options: &[],
            },
        )
    }

    #[test]
    fn f004_s03_exact_quote() {
        let quote = ask(
            date!(2027 - 05 - 03),
            date!(2027 - 05 - 06),
            2,
            1,
            &rules(),
            &offers(),
        )
        .unwrap();
        assert_eq!(
            quote,
            Quote {
                nights: 3,
                nights_total: 30_000,
                discount: 0,
                promotion: None,
                cleaning_fee: 6_000,
                options: Vec::new(),
                options_total: 0,
                stay_total: 36_000,
                deposit: 18_000,
                balance: 18_000,
                balance_due: Some(date!(2027 - 04 - 03)),
                tourist_tax: 1_122,
            }
        );
    }

    #[test]
    fn f004_s03_everything_due_at_booking_when_the_balance_date_is_reached() {
        let quote = ask(
            date!(2027 - 03 - 20),
            date!(2027 - 03 - 22),
            2,
            0,
            &rules(),
            &offers(),
        )
        .unwrap();
        assert_eq!(quote.deposit, 26_000);
        assert_eq!(quote.balance, 0);
        assert_eq!(quote.balance_due, None);
    }

    #[test]
    fn f004_s04_refusals_explain_why() {
        let mut nights = offers();
        nights[70].price = None; // 10 May 2027
        let refused = ask(
            date!(2027 - 05 - 09),
            date!(2027 - 05 - 12),
            2,
            0,
            &rules(),
            &nights,
        )
        .unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::Unavailable]);
        assert_eq!(refused[0].to_string(), "dates indisponibles");

        let refused = ask(
            date!(2027 - 05 - 03),
            date!(2027 - 05 - 04),
            4,
            2,
            &rules(),
            &offers(),
        )
        .unwrap_err();
        assert_eq!(
            refused,
            vec![QuoteRefusal::Capacity(5), QuoteRefusal::MinimumStay(2)]
        );
        assert_eq!(refused[0].to_string(), "5 personnes au maximum");
        assert_eq!(
            refused[1].to_string(),
            "2 nuits minimum pour cette date d'arrivée"
        );

        let refused = ask(
            date!(2027 - 05 - 03),
            date!(2027 - 05 - 03),
            2,
            0,
            &rules(),
            &offers(),
        )
        .unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::DatesOrder]);

        let refused = ask(
            date!(2027 - 05 - 03),
            date!(2027 - 05 - 06),
            0,
            2,
            &rules(),
            &offers(),
        )
        .unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::NoAdult]);
    }

    #[test]
    fn f004_s05_minimum_delay_and_unknown_nights() {
        let refused = ask(TODAY, date!(2027 - 03 - 03), 2, 0, &rules(), &offers()).unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::TooSoon(date!(2027 - 03 - 02))]);
        assert_eq!(refused[0].to_string(), "arrivée au plus tôt le 02/03/2027");
        // Nights the API did not offer (beyond the horizon) are unavailable.
        let refused = ask(
            date!(2027 - 05 - 30),
            date!(2027 - 06 - 02),
            2,
            0,
            &rules(),
            &offers(),
        )
        .unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::Unavailable]);
    }

    #[test]
    fn f004_s10_weekdays() {
        let saturday = Weekdays::from_bits(1 << 5).unwrap();
        let rules = HouseRules {
            arrival_weekdays: saturday,
            departure_weekdays: saturday,
            ..rules()
        };
        // Wednesday 5 May to Saturday 8 May.
        let refused = ask(
            date!(2027 - 05 - 05),
            date!(2027 - 05 - 08),
            2,
            0,
            &rules,
            &offers(),
        )
        .unwrap_err();
        assert_eq!(refused, vec![QuoteRefusal::ArrivalDay(saturday)]);
        assert_eq!(refused[0].to_string(), "arrivée possible : samedi");
        // Saturday 1 May to Saturday 8 May.
        assert!(
            ask(
                date!(2027 - 05 - 01),
                date!(2027 - 05 - 08),
                2,
                0,
                &rules,
                &offers()
            )
            .is_ok()
        );
    }

    fn linen() -> StayOption {
        StayOption {
            name: Localized {
                fr: "Linge de maison".to_owned(),
                en: String::new(),
            },
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
    fn f026_s01_s04_promotion_and_options_in_the_quote() {
        let week = Promotion {
            name: Localized {
                fr: "Semaine".to_owned(),
                en: String::new(),
            },
            discount: Discount::Percent { bp: 1000 },
            bounds: NightBounds {
                min_nights: Some(7),
                max_nights: None,
            },
            valid_from: None,
            valid_until: None,
            active: true,
        };
        let options = [(4, linen())];
        let rules = rules();
        let nights = offers();
        let context = QuoteContext {
            today: TODAY,
            rules: &rules,
            nights: &nights,
            cleaning_fee: 6_000,
            promotions: std::slice::from_ref(&week),
            options: &options,
        };
        let request = StayRequest {
            arrival: date!(2027 - 05 - 01),
            departure: date!(2027 - 05 - 08),
            adults: 2,
            children: 0,
            options: vec![4],
        };
        let quote = quote(&request, context).unwrap();
        assert_eq!(quote.nights_total, 70_000);
        assert_eq!(quote.discount, 7_000);
        assert_eq!(quote.promotion, Some(0));
        assert_eq!(
            quote.options,
            vec![QuotedOption {
                id: 4,
                price: 3_000,
                selected: true
            }]
        );
        assert_eq!(quote.options_total, 3_000);
        assert_eq!(quote.stay_total, 70_000 - 7_000 + 6_000 + 3_000);
        assert_eq!(quote.deposit, quote.stay_total / 2);

        // 2 nights: no promotion, the linen is not offered and cannot be ticked.
        let short = StayRequest {
            departure: date!(2027 - 05 - 03),
            ..request.clone()
        };
        assert_eq!(
            super::quote(&short, context).unwrap_err(),
            vec![QuoteRefusal::OptionNotOffered]
        );
        let short = StayRequest {
            options: Vec::new(),
            ..short
        };
        let quote = super::quote(&short, context).unwrap();
        assert_eq!((quote.discount, quote.promotion), (0, None));
        assert_eq!(quote.options, Vec::new());
    }
}
