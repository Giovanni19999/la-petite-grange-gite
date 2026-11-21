//! Unit tests of the calendar service (F005), with in-memory ports.
#![allow(clippy::unwrap_used)]

use std::sync::Mutex;

use time::macros::{date, datetime};

use super::*;
use crate::domain::calendar::BlockedKind;

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

#[derive(Default)]
struct InMemoryPeriods {
    periods: Mutex<Vec<StoredPeriod>>,
    events: Mutex<Vec<PeriodEvent>>,
}

impl BlockedPeriodRepository for InMemoryPeriods {
    fn list(&self, cottage: CottageId) -> BoxFuture<'_, anyhow::Result<Vec<StoredPeriod>>> {
        let list = self
            .periods
            .lock()
            .unwrap()
            .iter()
            .filter(|period| period.cottage == cottage)
            .cloned()
            .collect();
        Box::pin(async move { Ok(list) })
    }
    fn find(&self, id: PeriodId) -> BoxFuture<'_, anyhow::Result<Option<StoredPeriod>>> {
        let found = self
            .periods
            .lock()
            .unwrap()
            .iter()
            .find(|period| period.id == id)
            .cloned();
        Box::pin(async move { Ok(found) })
    }
    fn save<'a>(&'a self, change: &'a PeriodChange) -> BoxFuture<'a, anyhow::Result<PeriodId>> {
        let mut periods = self.periods.lock().unwrap();
        let id = change
            .id
            .unwrap_or_else(|| PeriodId(periods.len() as u64 + 100));
        periods.retain(|period| period.id != id);
        if let Some(after) = &change.after {
            periods.push(StoredPeriod {
                id,
                cottage: change.cottage,
                period: after.clone(),
                updated_at: change.at,
            });
        }
        self.events.lock().unwrap().push(PeriodEvent {
            period: id,
            action: change.action,
            before: change.before.clone(),
            after: change.after.clone(),
            author: Some(change.author),
            author_email: None,
            at: change.at,
        });
        Box::pin(async move { Ok(id) })
    }
    fn history(
        &self,
        _cottage: CottageId,
        limit: u32,
    ) -> BoxFuture<'_, anyhow::Result<Vec<PeriodEvent>>> {
        let mut events = self.events.lock().unwrap().clone();
        events.reverse();
        events.truncate(limit as usize);
        Box::pin(async move { Ok(events) })
    }
}

/// One stay from 5 to 9 June 2027 (bookings come with F007).
struct OneStay;
impl StayCalendar for OneStay {
    fn stays<'a>(
        &'a self,
        _cottage: CottageId,
        range: &'a DateRange,
    ) -> BoxFuture<'a, anyhow::Result<Vec<Stay>>> {
        let stay = Stay {
            label: "LPG-2027-0001".into(),
            range: DateRange {
                start: date!(2027 - 06 - 05),
                end: Some(date!(2027 - 06 - 09)),
            },
        };
        let found = if stay.range.overlaps(range) {
            vec![stay]
        } else {
            Vec::new()
        };
        Box::pin(async move { Ok(found) })
    }
}

fn service(stays: Arc<dyn StayCalendar>) -> CalendarService {
    CalendarService::new(
        CalendarPorts {
            cottages: Arc::new(OneCottage),
            periods: Arc::new(InMemoryPeriods::default()),
            stays,
            clock: Arc::new(FixedClock),
        },
        400,
    )
}

fn closure(start: Date, end: Option<Date>) -> BlockedPeriodInput {
    BlockedPeriodInput {
        start,
        end,
        kind: BlockedKind::Closure,
        label: None,
        people: None,
        reason: Some("Travaux".into()),
    }
}

#[tokio::test]
async fn f005_s01_closed_nights_are_unavailable() {
    let calendar = service(Arc::new(NoStays));
    calendar
        .create(
            closure(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15))),
            OWNER,
        )
        .await
        .unwrap();
    let nights = calendar
        .availability(date!(2027 - 03 - 01), date!(2027 - 04 - 01))
        .await
        .unwrap();
    assert_eq!(nights.len(), 5);
    assert_eq!(nights[0], date!(2027 - 03 - 10));
    assert_eq!(nights[4], date!(2027 - 03 - 14));
}

#[tokio::test]
async fn f005_s04_stay_conflict_is_refused_with_the_stay() {
    let calendar = service(Arc::new(OneStay));
    let error = calendar
        .create(
            closure(date!(2027 - 06 - 07), Some(date!(2027 - 06 - 12))),
            OWNER,
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "des séjours occupent déjà ces dates : LPG-2027-0001"
    );
}

#[tokio::test]
async fn f005_s06_overlap_names_the_other_period() {
    let calendar = service(Arc::new(NoStays));
    calendar
        .create(
            closure(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15))),
            OWNER,
        )
        .await
        .unwrap();
    let family = BlockedPeriodInput {
        kind: BlockedKind::Family,
        label: Some("Mamie et Papi".into()),
        ..closure(date!(2027 - 03 - 14), Some(date!(2027 - 03 - 20)))
    };
    let error = calendar.create(family, OWNER).await.unwrap_err();
    assert_eq!(
        error.to_string(),
        "ces dates chevauchent une autre période (du 10/03/2027 au 15/03/2027)"
    );
}

#[tokio::test]
async fn f005_s07_change_reopen_and_history() {
    let calendar = service(Arc::new(NoStays));
    let id = calendar
        .create(
            closure(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 15))),
            OWNER,
        )
        .await
        .unwrap();
    // A period may be changed over its own dates.
    calendar
        .update(
            id,
            closure(date!(2027 - 03 - 10), Some(date!(2027 - 03 - 12))),
            OWNER,
        )
        .await
        .unwrap();
    let nights = calendar
        .availability(date!(2027 - 03 - 01), date!(2027 - 04 - 01))
        .await
        .unwrap();
    assert_eq!(nights, vec![date!(2027 - 03 - 10), date!(2027 - 03 - 11)]);

    calendar.delete(id, OWNER).await.unwrap();
    assert_eq!(
        calendar
            .availability(date!(2027 - 03 - 01), date!(2027 - 04 - 01))
            .await
            .unwrap(),
        Vec::<Date>::new()
    );

    let history = calendar.history(10).await.unwrap();
    let actions: Vec<PeriodAction> = history.iter().map(|event| event.action).collect();
    assert_eq!(
        actions,
        vec![
            PeriodAction::Deleted,
            PeriodAction::Updated,
            PeriodAction::Created
        ]
    );
    assert!(history.iter().all(|event| event.author == Some(OWNER)));
    assert_eq!(
        history[1].before.as_ref().unwrap().end,
        Some(date!(2027 - 03 - 15))
    );
}

#[tokio::test]
async fn f005_s05_open_ended_closure() {
    let calendar = service(Arc::new(NoStays));
    calendar
        .create(closure(date!(2026 - 12 - 01), None), OWNER)
        .await
        .unwrap();
    let nights = calendar
        .availability(date!(2026 - 11 - 28), date!(2026 - 12 - 03))
        .await
        .unwrap();
    assert_eq!(nights, vec![date!(2026 - 12 - 01), date!(2026 - 12 - 02)]);
}

#[tokio::test]
async fn f005_s08_horizon_from_today_in_paris() {
    let calendar = service(Arc::new(NoStays));
    let nights = calendar
        .availability(date!(2028 - 11 - 19), date!(2028 - 11 - 23))
        .await
        .unwrap();
    assert_eq!(nights, vec![date!(2028 - 11 - 21), date!(2028 - 11 - 22)]);
}

#[tokio::test]
async fn availability_range_is_limited() {
    let calendar = service(Arc::new(NoStays));
    assert!(matches!(
        calendar
            .availability(date!(2027 - 01 - 01), date!(2028 - 06 - 01))
            .await,
        Err(CalendarError::BadRange(_))
    ));
    assert!(matches!(
        calendar
            .availability(date!(2027 - 01 - 02), date!(2027 - 01 - 01))
            .await,
        Err(CalendarError::BadRange(_))
    ));
}

#[tokio::test]
async fn unknown_period() {
    let calendar = service(Arc::new(NoStays));
    assert!(matches!(
        calendar.delete(PeriodId(999), OWNER).await,
        Err(CalendarError::NotFound)
    ));
}
