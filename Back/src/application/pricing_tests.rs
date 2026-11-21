//! Unit tests of the price service (F006), with in-memory ports.
#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use time::macros::{date, datetime};

use super::*;
use crate::application::calendar::Cottage;
use crate::domain::calendar::BookingHorizon;

const OWNER: UserId = UserId(1);

struct FixedClock;
impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        datetime!(2026-11-21 10:00 UTC)
    }
}

struct OneCottage;
impl CottageRepository for OneCottage {
    fn default_cottage(&self) -> BoxFuture<'_, anyhow::Result<Cottage>> {
        Box::pin(async {
            Ok(Cottage {
                id: CottageId(1),
                slug: "la-petite-grange".into(),
                name: "La Petite Grange".into(),
                horizon: BookingHorizon {
                    months: 24,
                    until: None,
                },
            })
        })
    }
}

struct InMemoryPrices {
    book: Mutex<PriceBook>,
    history: Mutex<Vec<PriceChange>>,
}

impl PricingRepository for InMemoryPrices {
    fn load(&self, _cottage: CottageId) -> BoxFuture<'_, anyhow::Result<PriceBook>> {
        let book = self.book.lock().unwrap().clone();
        Box::pin(async move { Ok(book) })
    }
    fn save<'a>(
        &'a self,
        _cottage: CottageId,
        book: &'a PriceBook,
        change: &'a PriceChange,
    ) -> BoxFuture<'a, anyhow::Result<()>> {
        let mut saved = book.clone();
        // Like the database: new ids at every save.
        for (index, range) in saved.ranges.iter_mut().enumerate() {
            range.id = index as u64 + 1;
        }
        *self.book.lock().unwrap() = saved;
        self.history.lock().unwrap().push(change.clone());
        Box::pin(async { Ok(()) })
    }
    fn history(
        &self,
        _cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PriceChange>>> {
        let mut history = self.history.lock().unwrap().clone();
        history.reverse();
        history.truncate(limit as usize);
        Box::pin(async move { Ok(history) })
    }
}

fn channel(code: &str, name: &str, commission_bp: u16, payment_fees_bp: u16, vat: bool) -> Channel {
    Channel {
        code: code.into(),
        name: name.into(),
        rates: ChannelRates {
            commission_bp,
            payment_fees_bp,
            vat_on_commission: vat,
        },
    }
}

fn price(listed_euros: u64, net_euros: u64) -> ChannelPrice {
    ChannelPrice {
        listed: listed_euros * 100,
        net: net_euros * 100,
    }
}

fn seed() -> (PricingService, Arc<InMemoryPrices>) {
    let repository = Arc::new(InMemoryPrices {
        book: Mutex::new(PriceBook {
            channels: vec![
                channel("direct", "Site", 0, 0, false),
                channel("airbnb", "Airbnb", 1550, 0, true),
                channel("booking", "Booking", 1500, 140, true),
            ],
            settings: PricingSettings {
                cleaning_fee: 0,
                default_min_nights: 2,
                commission_alert_points: 5,
            },
            defaults: BTreeMap::from([("direct".to_owned(), price(100, 100))]),
            ranges: Vec::new(),
        }),
        history: Mutex::new(Vec::new()),
    });
    let service = PricingService::new(
        PricingPorts {
            cottages: Arc::new(OneCottage),
            prices: repository.clone(),
            clock: Arc::new(FixedClock),
        },
        400,
    );
    (service, repository)
}

fn summer() -> RangeInput {
    RangeInput {
        start: date!(2027 - 07 - 06),
        end: date!(2027 - 09 - 01),
        label: Some("Été".into()),
        min_nights: 2,
    }
}

async fn site_price(service: &PricingService, night: Date) -> Option<ChannelPrice> {
    service
        .nights(night, night.next_day().unwrap())
        .await
        .unwrap()[0]
        .prices["direct"]
}

#[tokio::test]
async fn f006_s01_default_price_of_the_site() {
    let (service, _) = seed();
    service
        .set_default_prices(
            BTreeMap::from([("direct".to_owned(), Some(price(80, 79)))]),
            OWNER,
        )
        .await
        .unwrap();
    assert_eq!(
        site_price(&service, date!(2027 - 03 - 10)).await,
        Some(price(80, 79))
    );
}

#[tokio::test]
async fn f006_s02_range_prices_per_channel() {
    let (service, _) = seed();
    service
        .create_range(
            summer(),
            BTreeMap::from([
                ("direct".to_owned(), price(110, 108)),
                ("airbnb".to_owned(), price(130, 110)),
                ("booking".to_owned(), price(130, 109)),
            ]),
            OWNER,
        )
        .await
        .unwrap();
    let nights = service
        .nights(date!(2027 - 07 - 05), date!(2027 - 07 - 11))
        .await
        .unwrap();
    assert_eq!(nights[0].prices["direct"], Some(price(100, 100)));
    assert_eq!(
        nights[0].prices["booking"], None,
        "no Booking default price"
    );
    assert_eq!(nights[5].prices["direct"], Some(price(110, 108)));
    assert_eq!(nights[5].prices["airbnb"], Some(price(130, 110)));
}

#[tokio::test]
async fn f006_s03_new_range_cuts_the_others_and_keeps_their_prices() {
    let (service, repository) = seed();
    let july = RangeInput {
        start: date!(2027 - 07 - 01),
        end: date!(2027 - 07 - 31),
        label: Some("Juillet".into()),
        min_nights: 2,
    };
    service
        .create_range(
            july,
            BTreeMap::from([("direct".to_owned(), price(120, 120))]),
            OWNER,
        )
        .await
        .unwrap();
    let middle = RangeInput {
        start: date!(2027 - 07 - 10),
        end: date!(2027 - 07 - 20),
        label: None,
        min_nights: 3,
    };
    service
        .create_range(
            middle,
            BTreeMap::from([("direct".to_owned(), price(150, 150))]),
            OWNER,
        )
        .await
        .unwrap();
    let book = repository.book.lock().unwrap().clone();
    let ranges: Vec<(Date, Date, Option<u64>)> = book
        .ranges
        .iter()
        .map(|range| {
            (
                range.range.start,
                range.range.end,
                range.prices.get("direct").map(|price| price.listed),
            )
        })
        .collect();
    assert_eq!(
        ranges,
        vec![
            (date!(2027 - 07 - 01), date!(2027 - 07 - 10), Some(12_000)),
            (date!(2027 - 07 - 10), date!(2027 - 07 - 20), Some(15_000)),
            (date!(2027 - 07 - 20), date!(2027 - 07 - 31), Some(12_000)),
        ]
    );
}

#[tokio::test]
async fn f006_s05_invalid_prices_are_refused() {
    let (service, _) = seed();
    let error = service
        .set_default_prices(
            BTreeMap::from([("airbnb".to_owned(), Some(price(100, 120)))]),
            OWNER,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "prix invalides",
        "the detail is in the list"
    );
    assert!(matches!(
        service
            .create_range(summer(), BTreeMap::from([("vrbo".to_owned(), price(90, 80))]), OWNER)
            .await,
        Err(PriceBookError::Invalid(errors)) if errors == vec![PricingError::UnknownChannel("vrbo".into())]
    ));
}

#[tokio::test]
async fn f006_s06_incomplete_channels() {
    let (service, _) = seed();
    let book = service.book().await.unwrap();
    assert_eq!(
        PricingService::incomplete_channels(&book),
        vec!["airbnb".to_owned(), "booking".to_owned()]
    );
}

#[tokio::test]
async fn f006_s07_cleaning_fee_in_typical_stays() {
    let (service, _) = seed();
    service
        .set_settings(
            PricingSettings {
                cleaning_fee: 6_000,
                default_min_nights: 2,
                commission_alert_points: 5,
            },
            OWNER,
        )
        .await
        .unwrap();
    let book = service.book().await.unwrap();
    let stays = PricingService::typical_stays(&book, 7, None);
    assert_eq!(stays["direct"], Some((76_000, 76_000)));
    assert_eq!(stays["airbnb"], None);
}

#[tokio::test]
async fn f006_s08_minimum_nights() {
    let (service, _) = seed();
    service
        .create_range(
            RangeInput {
                start: date!(2027 - 07 - 15),
                end: date!(2027 - 07 - 22),
                label: None,
                min_nights: 3,
            },
            BTreeMap::new(),
            OWNER,
        )
        .await
        .unwrap();
    let july = service
        .nights(date!(2027 - 07 - 15), date!(2027 - 07 - 16))
        .await
        .unwrap();
    let june = service
        .nights(date!(2027 - 06 - 01), date!(2027 - 06 - 02))
        .await
        .unwrap();
    assert_eq!(july[0].min_nights, 3);
    assert_eq!(june[0].min_nights, 2);
}

#[tokio::test]
async fn f006_s09_copy_to_next_year() {
    let (service, repository) = seed();
    service
        .create_range(
            summer(),
            BTreeMap::from([("direct".to_owned(), price(110, 108))]),
            OWNER,
        )
        .await
        .unwrap();
    assert_eq!(service.copy_year(2027, OWNER).await.unwrap(), 1);
    let book = repository.book.lock().unwrap().clone();
    let copy = &book.ranges[1];
    assert_eq!(copy.range.start, date!(2028 - 07 - 04));
    assert_eq!(copy.range.label.as_deref(), Some("Été"));
    assert_eq!(copy.prices["direct"], price(110, 108));
}

#[tokio::test]
async fn f006_s10_copy_a_channel() {
    let (service, repository) = seed();
    service
        .create_range(
            summer(),
            BTreeMap::from([("direct".to_owned(), price(110, 108))]),
            OWNER,
        )
        .await
        .unwrap();
    service
        .copy_channel("direct", "airbnb", OWNER)
        .await
        .unwrap();
    let book = repository.book.lock().unwrap().clone();
    assert_eq!(book.defaults["airbnb"], price(100, 100));
    assert_eq!(book.ranges[0].prices["airbnb"], price(110, 108));
}

#[tokio::test]
async fn f006_s11_history() {
    let (service, _) = seed();
    service
        .set_default_prices(
            BTreeMap::from([
                ("direct".to_owned(), Some(price(90, 89))),
                ("airbnb".to_owned(), None),
            ]),
            OWNER,
        )
        .await
        .unwrap();
    let history = service.history(10).await.unwrap();
    assert_eq!(
        history[0].description,
        "Prix par défaut : Site 90 € / 89 €, Airbnb sans prix"
    );
    assert_eq!(history[0].author, Some(OWNER));
    assert_eq!(
        history[0].before.as_ref().unwrap()["direct"]["listed"],
        serde_json::json!(10_000)
    );
}

#[tokio::test]
async fn editing_and_deleting_ranges() {
    let (service, repository) = seed();
    service
        .create_range(summer(), BTreeMap::new(), OWNER)
        .await
        .unwrap();
    let id = repository.book.lock().unwrap().ranges[0].id;
    let shorter = RangeInput {
        end: date!(2027 - 08 - 01),
        ..summer()
    };
    service
        .update_range(id, shorter, BTreeMap::new(), OWNER)
        .await
        .unwrap();
    let id = repository.book.lock().unwrap().ranges[0].id;
    assert_eq!(
        repository.book.lock().unwrap().ranges[0].range.end,
        date!(2027 - 08 - 01)
    );
    service.delete_range(id, OWNER).await.unwrap();
    assert_eq!(repository.book.lock().unwrap().ranges.len(), 0);
    assert!(matches!(
        service.delete_range(999, OWNER).await,
        Err(PriceBookError::NotFound)
    ));
}

#[tokio::test]
async fn f006_s13_a_new_rate_never_changes_stored_prices() {
    let (service, repository) = seed();
    service
        .set_default_prices(
            BTreeMap::from([("booking".to_owned(), Some(price(140, 115)))]),
            OWNER,
        )
        .await
        .unwrap();
    service
        .set_channel_rates(
            "booking",
            ChannelRates {
                commission_bp: 1700,
                payment_fees_bp: 140,
                vat_on_commission: true,
            },
            OWNER,
        )
        .await
        .unwrap();
    let book = repository.book.lock().unwrap().clone();
    assert_eq!(book.defaults["booking"], price(140, 115));
    assert_eq!(book.channels[2].rates.commission_bp, 1700);
}
