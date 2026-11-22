//! Assembles the production adapters (composition, ADR-0002).
//!
//! [`Infrastructure`] owns the shared resources (database pool) and their lifecycle;
//! the free functions build use cases from them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use sqlx::MySqlPool;

use crate::app_state::AppState;
use crate::application::admin_accounts::AdminAccounts;
use crate::application::auth::{AuthPorts, AuthService, LoginThrottle};
use crate::application::bookings::{
    BookingPorts, BookingService, BookingServices, BookingSettings, NoPayment, PaymentGateway,
};
use crate::application::calendar::{CalendarPorts, CalendarService};
use crate::application::house_rules::HouseRulesService;
use crate::application::media::{MediaPorts, MediaService, MediaSettings};
use crate::application::messaging::{DocumentStore, Mailer, MessagingService, SmsSender};
use crate::application::offers::OffersService;
use crate::application::pages::PageService;
use crate::application::pricing::{PricingPorts, PricingService};
use crate::application::quote::QuoteService;
use crate::application::templates::TemplatesService;
use crate::config::{BookingConfig, Config, PaymentProvider, SmsProvider};
use crate::domain::auth::SessionPolicy;
use crate::infrastructure::auth::{
    Argon2Hasher, MariaDbSessions, MariaDbUsers, OsRandom, SystemClock,
};
use crate::infrastructure::bookings::{MariaDbBookings, MariaDbTemplates};
use crate::infrastructure::calendar::{MariaDbBlockedPeriods, MariaDbCottages};
use crate::infrastructure::documents::FsDocuments;
use crate::infrastructure::house_rules::MariaDbHouseRules;
use crate::infrastructure::images::WebpImageProcessor;
use crate::infrastructure::mariadb::{MariaDbProbe, lazy_pool, migrate};
use crate::infrastructure::media::MariaDbMedia;
use crate::infrastructure::media_files::FsMediaFiles;
use crate::infrastructure::messaging::MariaDbMessages;
use crate::infrastructure::offers::MariaDbOffers;
use crate::infrastructure::pages::MariaDbPages;
use crate::infrastructure::pricing::MariaDbPricing;
use crate::infrastructure::sms::{BrevoSms, NoSms};
use crate::infrastructure::smtp::SmtpMailer;

/// Providers of the email and SMS sending (F025, ADR-0019).
#[derive(Clone)]
pub struct Senders {
    pub mailer: Arc<dyn Mailer>,
    pub sms: Arc<dyn SmsSender>,
}

impl std::fmt::Debug for Senders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Senders").finish_non_exhaustive()
    }
}

impl Senders {
    /// SMTP with the password of the environment, and the SMS provider of the configuration.
    ///
    /// # Errors
    ///
    /// The Brevo HTTP client cannot be built.
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let sms: Arc<dyn SmsSender> = match &config.messaging.sms {
            SmsProvider::None => Arc::new(NoSms),
            SmsProvider::Brevo { api_key, base_url } => {
                Arc::new(BrevoSms::new(base_url, api_key.clone())?)
            }
        };
        Ok(Self {
            mailer: Arc::new(SmtpMailer::new(config.messaging.smtp_password.clone())),
            sms,
        })
    }
}

/// What to do when the migrations cannot be applied at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Migrations {
    /// Log the error and continue (the server starts, health reports the database down).
    BestEffort,
    /// Fail (one-shot commands need a ready database).
    Required,
}

/// Shared infrastructure resources and their lifecycle.
#[derive(Debug, Clone)]
pub struct Infrastructure {
    pool: MySqlPool,
    config: Config,
}

impl Infrastructure {
    /// Creates the database pool and applies the pending migrations.
    ///
    /// # Errors
    ///
    /// Returns an error when `DATABASE_URL` is invalid, or when the migrations fail and
    /// `migrations` is [`Migrations::Required`].
    pub async fn start(config: &Config, migrations: Migrations) -> anyhow::Result<Self> {
        let pool = lazy_pool(config).context("invalid DATABASE_URL")?;
        if let Err(error) = migrate(&pool).await {
            match migrations {
                Migrations::Required => {
                    return Err(error).context("cannot apply database migrations");
                }
                Migrations::BestEffort => {
                    tracing::error!(%error, "database migrations not applied");
                }
            }
        }
        Ok(Self {
            pool,
            config: config.clone(),
        })
    }

    /// State of the HTTP adapter.
    ///
    /// # Errors
    ///
    /// Returns an error if a service cannot be initialised.
    pub fn app_state(&self) -> anyhow::Result<AppState> {
        let mut state = app_state(
            &self.pool,
            &StateSettings::from_config(&self.config),
            Senders::from_config(&self.config)?,
        )?;
        state.trust_proxy_headers = self.config.trust_proxy_headers;
        if self.config.serve_media_files {
            state.serve_media_dir = Some(self.config.media_dir.join("public"));
        }
        Ok(state)
    }

    /// Starts the background task that sends the queued messages (F025, ADR-0019) and
    /// alerts the owners of late deposits (F007).
    #[must_use]
    pub fn start_message_worker(&self, state: &AppState) -> tokio::task::JoinHandle<()> {
        spawn_message_worker(
            state.messaging.clone(),
            state.bookings.clone(),
            Duration::from_secs(u64::from(self.config.messaging.worker_interval_seconds)),
        )
    }

    /// Use case of the command-line adapter.
    ///
    /// # Errors
    ///
    /// Returns an error if the password hasher cannot be initialised.
    pub fn admin_accounts(&self) -> anyhow::Result<AdminAccounts> {
        admin_accounts(&self.pool)
    }

    /// Closes the database connections.
    pub async fn shutdown(self) {
        self.pool.close().await;
    }
}

/// Builds the authentication service backed by `MariaDB`.
///
/// # Errors
///
/// Returns an error if the password hasher cannot be initialised.
pub fn auth_service(
    pool: &MySqlPool,
    policy: SessionPolicy,
    throttle: LoginThrottle,
) -> anyhow::Result<AuthService> {
    let ports = AuthPorts {
        users: Arc::new(MariaDbUsers::new(pool.clone())),
        sessions: Arc::new(MariaDbSessions::new(pool.clone())),
        hasher: Arc::new(Argon2Hasher::new()?),
        clock: Arc::new(SystemClock),
        random: Arc::new(OsRandom),
    };
    Ok(AuthService::new(ports, policy, throttle))
}

/// Builds the admin account management use case backed by `MariaDB`.
///
/// # Errors
///
/// Returns an error if the password hasher cannot be initialised.
pub fn admin_accounts(pool: &MySqlPool) -> anyhow::Result<AdminAccounts> {
    Ok(AdminAccounts::new(
        Arc::new(MariaDbUsers::new(pool.clone())),
        Arc::new(Argon2Hasher::new()?),
    ))
}

/// Builds the media library service: `MariaDB` records, files under `media_dir`.
///
/// # Errors
///
/// Returns an error if the media directories cannot be created.
pub fn media_service(
    pool: &MySqlPool,
    media_dir: &Path,
    settings: MediaSettings,
) -> anyhow::Result<MediaService> {
    Ok(MediaService::new(
        MediaPorts {
            media: Arc::new(MariaDbMedia::new(pool.clone())),
            files: Arc::new(FsMediaFiles::new(media_dir)?),
            processor: Arc::new(WebpImageProcessor),
            pages: Arc::new(MariaDbPages::new(pool.clone())),
            clock: Arc::new(SystemClock),
        },
        settings,
    ))
}

/// Builds the calendar service backed by `MariaDB`; the stays are the direct bookings (F007;
/// imported stays come with F014).
#[must_use]
pub fn calendar_service(pool: &MySqlPool, availability_max_days: u32) -> CalendarService {
    CalendarService::new(
        CalendarPorts {
            cottages: Arc::new(MariaDbCottages::new(pool.clone())),
            periods: Arc::new(MariaDbBlockedPeriods::new(pool.clone())),
            stays: Arc::new(MariaDbBookings::new(pool.clone())),
            clock: Arc::new(SystemClock),
        },
        availability_max_days,
    )
}

/// How often late deposits are looked for (F007).
const DEPOSIT_CHECK: Duration = Duration::from_secs(3600);

/// Background task: marks the messages interrupted by a stop `to_check`, then sends the due
/// messages every `interval` and looks for late deposits every hour, until aborted.
#[must_use]
pub fn spawn_message_worker(
    messaging: Arc<MessagingService>,
    bookings: Arc<BookingService>,
    interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match messaging.recover_interrupted().await {
            Ok(0) => {}
            Ok(count) => tracing::warn!(count, "interrupted messages marked to check"),
            Err(error) => tracing::error!(%error, "cannot recover the interrupted messages"),
        }
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_check: Option<tokio::time::Instant> = None;
        loop {
            ticker.tick().await;
            if last_check.is_none_or(|at| at.elapsed() >= DEPOSIT_CHECK) {
                last_check = Some(tokio::time::Instant::now());
                match bookings.alert_late_deposits().await {
                    Ok(0) => {}
                    Ok(count) => tracing::info!(count, "late deposits alerted"),
                    Err(error) => tracing::error!(%error, "cannot look for late deposits"),
                }
            }
            if let Err(error) = messaging.process_due().await {
                tracing::error!(%error, "message sending round failed");
            }
        }
    })
}

/// Technical settings the application state is built from.
#[derive(Debug, Clone)]
pub struct StateSettings {
    pub policy: SessionPolicy,
    pub throttle: LoginThrottle,
    pub media_dir: PathBuf,
    pub media: MediaSettings,
    pub availability_max_days: u32,
    pub bookings: BookingConfig,
}

impl StateSettings {
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            policy: config.session_policy,
            throttle: config.login_throttle,
            media_dir: config.media_dir.clone(),
            media: config.media.clone(),
            availability_max_days: config.availability_max_days,
            bookings: config.bookings.clone(),
        }
    }
}

/// Builds the full application state.
///
/// # Errors
///
/// Returns an error if a service cannot be initialised.
pub fn app_state(
    pool: &MySqlPool,
    settings: &StateSettings,
    senders: Senders,
) -> anyhow::Result<AppState> {
    let availability_max_days = settings.availability_max_days;
    let media = Arc::new(media_service(
        pool,
        &settings.media_dir,
        settings.media.clone(),
    )?);
    let documents: Arc<dyn DocumentStore> = Arc::new(FsDocuments::new(&settings.media_dir)?);
    let calendar = Arc::new(calendar_service(pool, availability_max_days));
    let pricing = Arc::new(PricingService::new(
        PricingPorts {
            cottages: Arc::new(MariaDbCottages::new(pool.clone())),
            prices: Arc::new(MariaDbPricing::new(pool.clone())),
            clock: Arc::new(SystemClock),
        },
        availability_max_days,
    ));
    let house_rules = Arc::new(HouseRulesService::new(
        Arc::new(MariaDbCottages::new(pool.clone())),
        Arc::new(MariaDbHouseRules::new(pool.clone())),
        Arc::new(SystemClock),
    ));
    let offers = Arc::new(OffersService::new(
        Arc::new(MariaDbCottages::new(pool.clone())),
        Arc::new(MariaDbOffers::new(pool.clone())),
        Arc::new(SystemClock),
    ));
    let quotes = Arc::new(QuoteService::new(
        calendar.clone(),
        pricing.clone(),
        house_rules.clone(),
        offers.clone(),
        Arc::new(SystemClock),
    ));
    let messaging = Arc::new(MessagingService::new(
        Arc::new(MariaDbMessages::new(pool.clone())),
        senders.mailer,
        senders.sms,
        documents.clone(),
        Arc::new(SystemClock),
    ));
    let templates = Arc::new(TemplatesService::new(Arc::new(MariaDbTemplates::new(
        pool.clone(),
    ))));
    let bookings_db = Arc::new(MariaDbBookings::new(pool.clone()));
    let payments: Arc<dyn PaymentGateway> = match settings.bookings.payment_provider {
        PaymentProvider::None => Arc::new(NoPayment),
    };
    let bookings = Arc::new(BookingService::new(
        BookingPorts {
            bookings: bookings_db.clone(),
            payment_info: bookings_db.clone(),
            terms: bookings_db.clone(),
            attempts: bookings_db,
            documents,
            payments,
            cottages: Arc::new(MariaDbCottages::new(pool.clone())),
            clock: Arc::new(SystemClock),
        },
        BookingServices {
            quotes: quotes.clone(),
            rules: house_rules.clone(),
            messaging: messaging.clone(),
            templates: templates.clone(),
        },
        BookingSettings {
            site_url: settings.bookings.site_url.clone(),
            max_attempts: settings.bookings.max_attempts,
            attempt_window: settings.bookings.attempt_window,
        },
    ));
    Ok(AppState {
        database: Arc::new(MariaDbProbe::new(pool.clone())),
        auth: Arc::new(auth_service(pool, settings.policy, settings.throttle)?),
        pages: Arc::new(PageService::new(
            Arc::new(MariaDbPages::new(pool.clone())),
            Arc::new(SystemClock),
            media.clone(),
        )),
        media,
        calendar,
        pricing,
        house_rules,
        offers,
        quotes,
        messaging,
        templates,
        bookings,
        serve_media_dir: None,
        trust_proxy_headers: false,
    })
}
