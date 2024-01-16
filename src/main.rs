use chrono::Utc;
use consts::{CSFLOAT_ONE_LISTING_REQ_INTERVAL, DB_SAVE_INTERVAL};
use dotenvy::dotenv;
use reqwest::Client;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use teloxide::Bot;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tracing::{error, info, level_filters::LevelFilter, trace, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use types::ListingId;

mod business_logic;
mod consts;
mod csfloat;
mod csfloat_autobuy;
mod event_processors;
mod events;
mod fee;
mod models;
mod prices;
mod realtime_importer;
mod stats;
mod steam_analyzer;
mod storages;
mod types;
mod utils;

#[cfg(test)]
mod tests;

use event_processors::{
    process_csfloat_listings_response, process_profitable_listing, process_steam_response,
    process_updated_csfloat_listing,
};
use events::{CsfloatResponseEvent, Event, PrimEvent, SecEvent, SteamResponseEvent};
use realtime_importer::RealtimeImporter;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use stats::Stats;
use storages::{CsfloatEngine, SteamEngine};

use crate::csfloat_autobuy::CsfloatAutobuy;
use crate::prices::PriceValueTrait;
use crate::{
    csfloat::CsfloatScheduler,
    event_processors::process_csfloat_one_listing_response,
    events::CsfloatOneListingResponseEvent,
    stats::StatsKind,
    storages::{CsfloatEngineTrait, DbSerializable},
};

fn spawn_primary_event_dispatcher(
    prim_tx: Sender<PrimEvent>,
    sec_tx: Sender<SecEvent>,
    mut prim_rx: Receiver<PrimEvent>,
    stats: Arc<Mutex<Stats>>,
    csfloat_engine: Arc<Mutex<CsfloatEngine>>,
    steam_engine: Arc<Mutex<SteamEngine>>,
    csfloat_scheduler: Arc<Mutex<CsfloatScheduler>>,
) {
    tokio::spawn(async move {
        while let Some(event) = prim_rx.recv().await {
            let _start = Instant::now();

            let mut csfloat_engine_locked = csfloat_engine.lock().await;
            let mut steam_engine_locked = steam_engine.lock().await;
            let mut csfloat_scheduler_locked = csfloat_scheduler.lock().await;

            let _duration_before = _start.elapsed();
            if _duration_before.as_micros() > 1 {
                warn!("Waited {:?} to lock 3 mutexes", _duration_before);
            }

            // Dispatch events to their respective processing functions
            let new_events = match event {
                PrimEvent::CsfloatListingsResponse(ref e) => {
                    process_csfloat_listings_response(
                        &mut csfloat_engine_locked,
                        &mut csfloat_scheduler_locked,
                        e,
                    )
                    .await
                }
                PrimEvent::CsfloatOneListingResponse(ref e) => {
                    process_csfloat_one_listing_response(
                        &mut csfloat_engine_locked,
                        &mut csfloat_scheduler_locked,
                        e,
                    )
                    .await
                }
                PrimEvent::SteamResponse(ref e) => {
                    process_steam_response(&mut steam_engine_locked, e).await
                }
                PrimEvent::UpdatedCsfloatListings(ref e) => {
                    process_updated_csfloat_listing(
                        &mut steam_engine_locked,
                        &mut csfloat_engine_locked,
                        e,
                    )
                    .await
                }
            };

            for new_event in new_events {
                match new_event {
                    Event::Primary(prim_event) => {
                        let res = prim_tx.try_send(prim_event);
                        if res.is_err() {
                            error!("Failed to sent new event in the queue!");
                        }
                    }
                    Event::Secondary(sec_event) => {
                        let res = sec_tx.try_send(sec_event);
                        if res.is_err() {
                            error!("Failed to sent new event in the queue!");
                        }
                    }
                };
            }

            let _duration = _start.elapsed();
            let mut stats_locked = stats.lock().await;

            match &event {
                PrimEvent::CsfloatOneListingResponse(_) => {
                    stats_locked.register_duration(StatsKind::CsfloatOneListingResponse, _duration);
                }
                PrimEvent::CsfloatListingsResponse(_) => {
                    stats_locked.register_duration(StatsKind::CsfloatListingsResponse, _duration);
                }
                PrimEvent::SteamResponse(_) => {
                    stats_locked.register_duration(StatsKind::SteamResponse, _duration);
                }
                PrimEvent::UpdatedCsfloatListings(_) => {
                    stats_locked.register_duration(StatsKind::UpdatedCsfloatListings, _duration);
                }
            }
        }
    });
}

fn spawn_secondary_event_dispatcher(
    prim_tx: Sender<PrimEvent>,
    sec_tx: Sender<SecEvent>,
    mut sec_rx: Receiver<SecEvent>,
    bot: Bot,
    stats: Arc<Mutex<Stats>>,
    csfloat_autobuy: Arc<Mutex<CsfloatAutobuy>>,
) {
    tokio::spawn(async move {
        while let Some(event) = sec_rx.recv().await {
            let _start = Instant::now();

            let mut csfloat_autobuy_locked = csfloat_autobuy.lock().await;

            // Dispatch events to their respective processing functions
            let new_events = match event {
                SecEvent::ProfitableListing(ref e) => {
                    process_profitable_listing(&bot, &mut csfloat_autobuy_locked, e).await
                }
            };

            for new_event in new_events {
                // tx_clone.send(new_event).await.expect("Error sending event");
                match new_event {
                    Event::Primary(prim_event) => {
                        let res = prim_tx.try_send(prim_event);
                        if res.is_err() {
                            error!("Failed to sent new event in the queue!");
                        }
                    }
                    Event::Secondary(sec_event) => {
                        let res = sec_tx.try_send(sec_event);
                        if res.is_err() {
                            error!("Failed to sent new event in the queue!");
                        }
                    }
                };
            }

            let _duration = _start.elapsed();

            let mut stats_locked = stats.lock().await;
            match &event {
                SecEvent::ProfitableListing(_) => {
                    stats_locked.register_duration(StatsKind::ProfitableListing, _duration);
                }
            }
        }
    });
}

fn spawn_importer(pool: Pool<Postgres>, tx: Sender<PrimEvent>) {
    tokio::spawn(async move {
        let mut ri = RealtimeImporter::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            for csfloat_response in ri.get_csfloat_new(&pool, 8).await {
                let csfloat_response_event = CsfloatResponseEvent {
                    timestamp: Instant::now(),
                    response: csfloat_response,
                };
                tx.send(PrimEvent::CsfloatListingsResponse(csfloat_response_event))
                    .await
                    .expect("Error sending event");
            }

            for steam_response in ri.get_steam_new(&pool, 8).await {
                let steam_response_event = SteamResponseEvent {
                    timestamp: Utc::now(),
                    response: steam_response,
                };
                tx.send(PrimEvent::SteamResponse(steam_response_event))
                    .await
                    .expect("Error sending event");
            }
        }
    });
}

fn spawn_csfloat_refresher(tx: Sender<PrimEvent>, csfloat_scheduler: Arc<Mutex<CsfloatScheduler>>) {
    tokio::spawn(async move {
        let client = Client::new();

        loop {
            tokio::time::sleep(CSFLOAT_ONE_LISTING_REQ_INTERVAL).await;

            let next: Option<ListingId>;
            {
                let mut csfloat_scheduler_locked = csfloat_scheduler.lock().await;
                next = csfloat_scheduler_locked.get_next();
                if let Some(listing_id) = &next {
                    trace!(
                        "csfloat_scheduler size: {} | next was: {:?}",
                        csfloat_scheduler_locked.get_size(),
                        *listing_id
                    );
                }
            }

            if let Some(listing_id) = next {
                let url = format!("https://csfloat.com/api/v1/listings/{}", listing_id);
                let response = match client.get(&url).send().await {
                    Ok(x) => Some(x),
                    Err(_) => None,
                };

                if let Some(response) = response {
                    let text = match response.text().await {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    };
                    if text.is_some() {
                        let csfloat_response_event = CsfloatOneListingResponseEvent {
                            timestamp: Instant::now(),
                            response: text.unwrap(),
                        };
                        let new_event =
                            PrimEvent::CsfloatOneListingResponse(csfloat_response_event);
                        let res = tx.try_send(new_event);
                        if res.is_err() {
                            error!("Failed to sent new event in the queue!");
                        }
                    }
                }
            }
        }
    });
}

fn spawn_db_saver(
    pool: Pool<Postgres>,
    stats: Arc<Mutex<Stats>>,
    csfloat_engine: Arc<Mutex<CsfloatEngine>>,
    steam_engine: Arc<Mutex<SteamEngine>>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(DB_SAVE_INTERVAL);
        loop {
            interval.tick().await;

            {
                let stats_locked = stats.lock().await;
                stats_locked.print();
            }

            let csfloat_engine = csfloat_engine.lock().await;
            let steam_engine = steam_engine.lock().await;
            let csfloat_size = csfloat_engine.hm.len();
            let steam_size = steam_engine.hm.len();

            let _start = Instant::now();
            csfloat_engine.serialize(&pool).await;
            steam_engine.serialize(&pool).await;

            let _duration = _start.elapsed();

            info!("Dumped state to DB in {:?}", _duration);

            info!(
                "Data saved to the database at {:?} | csfloat size {} | steam size {}",
                Utc::now(),
                csfloat_size,
                steam_size
            );
        }
    });
}

fn init_logging() -> Result<WorkerGuard, Box<dyn std::error::Error>> {
    fn get_filter() -> Result<EnvFilter, Box<dyn std::error::Error>> {
        Ok(EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env()?)
    }

    let file_appender = tracing_appender::rolling::hourly("logs/", "prefix.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());
    // tracing_subscriber::fmt().with_writer(file_appender).with_writer(non_blocking).init();
    let file_filter = get_filter()?;
    let console_filter = get_filter()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false)
                .with_filter(file_filter),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(true)
                .with_filter(console_filter),
        )
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let _guard = init_logging()?;

    info!("Starting the program...");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    info!("Database URL is {}", database_url);
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Create an asynchronous channels for event communication
    const PRIMARY_QUEUE_SIZE: usize = 64_000;
    const SECONDARY_QUEUE_SIZE: usize = 64_000;
    let (prim_tx, prim_rx) = mpsc::channel::<PrimEvent>(PRIMARY_QUEUE_SIZE);
    let (sec_tx, sec_rx) = mpsc::channel::<SecEvent>(SECONDARY_QUEUE_SIZE);

    let csfloat_engine_itself = CsfloatEngine::deserialize(&pool).await;
    let steam_engine_itself = SteamEngine::deserialize(&pool).await;
    let mut csfloat_scheduler_itself = CsfloatScheduler::new();
    for listing in csfloat_engine_itself.get_listing_ids_by_update_time() {
        csfloat_scheduler_itself.upsert_listing(&listing);
    }

    assert_eq!(
        csfloat_engine_itself.get_size(),
        csfloat_scheduler_itself.get_size()
    );

    info!(
        "Loaded state for CsfloatEngine: {} | SteamEngine: {}",
        csfloat_engine_itself.hm.len(),
        steam_engine_itself.hm.len()
    );
    let csfloat_engine = Arc::new(Mutex::new(csfloat_engine_itself));
    let steam_engine = Arc::new(Mutex::new(steam_engine_itself));
    let csfloat_scheduler = Arc::new(Mutex::new(csfloat_scheduler_itself));
    let stats = Arc::new(Mutex::new(Stats::new()));

    let csfloat_autobuy = Arc::new(Mutex::new(CsfloatAutobuy::from_env()));
    let bot = Bot::from_env();

    {
        let mut csfloat_autobuy_locked = csfloat_autobuy.lock().await;
        let balance = csfloat_autobuy_locked.get_balance().await?;
        warn!("Csfloat balance is ${}", balance.to_usd());
    }

    // Start the event dispatchers
    spawn_primary_event_dispatcher(
        prim_tx.clone(),
        sec_tx.clone(),
        prim_rx,
        stats.clone(),
        csfloat_engine.clone(),
        steam_engine.clone(),
        csfloat_scheduler.clone(),
    );

    spawn_secondary_event_dispatcher(
        prim_tx.clone(),
        sec_tx.clone(),
        sec_rx,
        bot.clone(),
        stats.clone(),
        csfloat_autobuy.clone(),
    );

    spawn_importer(pool.clone(), prim_tx.clone());

    spawn_csfloat_refresher(prim_tx.clone(), csfloat_scheduler.clone());

    spawn_db_saver(
        pool,
        stats.clone(),
        csfloat_engine.clone(),
        steam_engine.clone(),
    );

    loop {
        // Perform other tasks or sleep here
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
