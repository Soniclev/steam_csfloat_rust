use std::time::Instant;

use chrono::{DateTime, Utc};

use crate::{
    prices::PriceValue,
    types::{ListingId, MarketName},
};

#[derive(Debug, PartialEq)]
pub struct CsfloatResponseEvent {
    pub timestamp: Instant,
    pub response: String,
}

#[derive(Debug, PartialEq)]
pub struct CsfloatOneListingResponseEvent {
    pub timestamp: Instant,
    pub response: String,
}

#[derive(Debug, PartialEq)]
pub struct SteamResponseEvent {
    pub timestamp: DateTime<Utc>,
    pub response: String,
}

#[derive(Debug, PartialEq)]
pub struct UpdatedCsfloatListingsEvent {
    pub listing_ids: Vec<ListingId>,
}

#[derive(Debug, PartialEq)]
pub enum PrimEvent {
    // primary events
    CsfloatOneListingResponse(CsfloatOneListingResponseEvent),
    CsfloatListingsResponse(CsfloatResponseEvent),
    SteamResponse(SteamResponseEvent),
    UpdatedCsfloatListings(UpdatedCsfloatListingsEvent),
    // secondary events
}

#[derive(Debug, PartialEq)]
pub enum ProfitableListingKind {
    Profitable,
    GoodPhase,
}

#[derive(Debug, PartialEq)]
pub struct ProfitableListingEvent {
    pub kind: ProfitableListingKind,
    pub market_name: MarketName,
    pub listing_id: ListingId,
    pub csfloat_price: PriceValue,
    pub steam_price: PriceValue,
    pub steam_no_fee: PriceValue,
    pub sold_per_week: u64,
    pub is_stable: bool,
    pub profit_pct: f64,
    pub float: Option<f64>,
}

#[derive(Debug, PartialEq)]
pub enum SecEvent {
    // secondary events
    ProfitableListing(ProfitableListingEvent),
}

#[derive(Debug, PartialEq)]
pub enum Event {
    Primary(PrimEvent),
    Secondary(SecEvent),
}
