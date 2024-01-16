use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use tracing::{error, warn};

use crate::{
    models::{CsfloatListingState, CsfloatListingStruct},
    steam_analyzer::AnalysisResult,
    types::{ListingId, MarketName},
};

pub trait DbSerializable<T> {
    async fn deserialize(db: &Pool<Postgres>) -> T;
    async fn serialize(&self, db: &Pool<Postgres>);
    async fn deserialize_load(db: &Pool<Postgres>, key: &str) -> Option<String> {
        match sqlx::query_scalar("SELECT value FROM rust_dump WHERE key = $1")
            .bind(key)
            .fetch_one(db)
            .await
        {
            Ok(it) => it,
            Err(err) => {
                match err {
                    sqlx::Error::RowNotFound => warn!("No saved state for {}", key),
                    _ => {
                        warn!("Failed to load state for {} {:?}", key, err);
                    }
                }

                None
            }
        }
    }
    async fn serialize_to_db(db: &Pool<Postgres>, key: &str, serialized: String) {
        match sqlx::query(
            "INSERT INTO rust_dump (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2",
        )
        .bind(key)
        .bind(serialized)
        .execute(db)
        .await
        {
            Ok(_) => {}
            Err(err) => error!(
                "Failed to serialize and save state for {}: {:?}",
                key, err
            ),
        };
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CsfloatEngine {
    pub hm: HashMap<ListingId, CsfloatListingStruct>,
    pub listing_id_to_last_update_time: HashMap<ListingId, Option<DateTime<Utc>>>,
}

impl CsfloatEngine {
    pub fn new() -> Self {
        CsfloatEngine {
            hm: HashMap::new(),
            listing_id_to_last_update_time: HashMap::new(),
        }
    }
}

pub enum CsfloatEngineListingDecision {
    New,
    NotChanged,
    Updated,
    Removed,
}

pub trait CsfloatEngineTrait {
    fn get_size(&self) -> usize;
    fn get_listing_ids_by_update_time(&self) -> Vec<ListingId>;
    fn remove_listing(&mut self, listing_id: &ListingId);
    fn update_listing(
        &mut self,
        listing_struct: &CsfloatListingStruct,
    ) -> CsfloatEngineListingDecision;
}

impl CsfloatEngineTrait for CsfloatEngine {
    fn get_size(&self) -> usize {
        self.hm.len()
    }

    fn get_listing_ids_by_update_time(&self) -> Vec<ListingId> {
        let mut result: Vec<(&String, &Option<DateTime<Utc>>)> =
            self.listing_id_to_last_update_time.iter().collect();
        result.sort_unstable_by_key(|x| *x.1);
        result.into_iter().map(|x| x.0.to_string()).collect()
    }

    fn update_listing(
        &mut self,
        listing_struct: &CsfloatListingStruct,
    ) -> CsfloatEngineListingDecision {
        let listing_id = &listing_struct.id;
        match self.hm.insert(listing_id.clone(), listing_struct.clone()) {
            Some(old_listing) => {
                if listing_struct.state == CsfloatListingState::Delisted
                    || listing_struct.state == CsfloatListingState::Sold
                    || listing_struct.state == CsfloatListingState::Refunded
                {
                    self.remove_listing(listing_id);
                    return CsfloatEngineListingDecision::Removed;
                }
                self.listing_id_to_last_update_time
                    .insert(listing_id.to_string(), Some(Utc::now()));
                let is_updated = old_listing.has_any_important_changes(listing_struct);
                match is_updated {
                    true => CsfloatEngineListingDecision::Updated,
                    false => CsfloatEngineListingDecision::NotChanged,
                }
            }
            None => {
                self.listing_id_to_last_update_time
                    .insert(listing_id.to_string(), Some(Utc::now()));
                CsfloatEngineListingDecision::New
            }
        }
    }

    fn remove_listing(&mut self, listing_id: &ListingId) {
        self.hm.remove(listing_id);
        self.listing_id_to_last_update_time.remove(listing_id);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SteamEngine {
    pub hm: HashMap<MarketName, AnalysisResult>,
}

impl SteamEngine {
    pub fn new() -> Self {
        SteamEngine { hm: HashMap::new() }
    }
}

pub trait SteamEngineTrait {
    fn update(&mut self, market_name: &MarketName, result: AnalysisResult);
}

impl SteamEngineTrait for SteamEngine {
    fn update(&mut self, market_name: &MarketName, result: AnalysisResult) {
        self.hm.insert(market_name.to_string(), result);
    }
}

const CSFLOAT_KEY: &str = "csfloat_engine";
const STEAM_KEY: &str = "steam_engine";

impl DbSerializable<CsfloatEngine> for CsfloatEngine {
    async fn deserialize(db: &Pool<Postgres>) -> CsfloatEngine {
        let value =
            <CsfloatEngine as DbSerializable<CsfloatEngine>>::deserialize_load(db, CSFLOAT_KEY)
                .await;
        if let Some(encoded) = value {
            let engine = match serde_json::from_str::<CsfloatEngine>(&encoded) {
                Ok(engine) => Some(engine),
                Err(err) => {
                    error!("Failed to deserialize state for CsfloatEngine: {}", err);
                    None
                }
            };
            return match engine {
                Some(engine) => engine,
                None => CsfloatEngine::new(),
            };
        }
        CsfloatEngine::new()
    }

    async fn serialize(&self, db: &Pool<Postgres>) {
        let serialized = serde_json::to_string(self).unwrap();
        <CsfloatEngine as DbSerializable<CsfloatEngine>>::serialize_to_db(
            db,
            CSFLOAT_KEY,
            serialized,
        )
        .await;
    }
}

impl DbSerializable<SteamEngine> for SteamEngine {
    async fn deserialize(db: &Pool<Postgres>) -> SteamEngine {
        let value =
            <SteamEngine as DbSerializable<SteamEngine>>::deserialize_load(db, STEAM_KEY).await;
        if let Some(encoded) = value {
            let engine = match serde_json::from_str::<SteamEngine>(&encoded) {
                Ok(engine) => Some(engine),
                Err(err) => {
                    error!("Failed to deserialize state for SteamEngine: {}", err);
                    None
                }
            };
            return match engine {
                Some(engine) => engine,
                None => SteamEngine::new(),
            };
        }
        SteamEngine::new()
    }

    async fn serialize(&self, db: &Pool<Postgres>) {
        let serialized = serde_json::to_string(self).unwrap();
        <SteamEngine as DbSerializable<SteamEngine>>::serialize_to_db(db, STEAM_KEY, serialized)
            .await;
    }
}
