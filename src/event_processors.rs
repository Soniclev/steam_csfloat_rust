use std::time::Duration;

use lazy_static::lazy_static;
use regex::Regex;
use teloxide::{requests::Requester, types::Recipient, Bot};
use tracing::{error, warn};

use crate::{
    business_logic::{
        is_good_glock_phase_listing, is_need_notify_via_telegram, is_need_to_autobuy,
        prefilter_listing,
    },
    consts::{DESIRED_PERCENTILE, IS_AUTOBUY_ALLOWED, MY_TG_ID},
    csfloat::CsfloatScheduler,
    csfloat_autobuy::CsfloatAutobuy,
    events::{
        CsfloatOneListingResponseEvent, CsfloatResponseEvent, Event, PrimEvent,
        ProfitableListingEvent, ProfitableListingKind, SecEvent, SteamResponseEvent,
        UpdatedCsfloatListingsEvent,
    },
    fee::SteamFee,
    models::CsfloatListingStruct,
    prices::{PriceValue, PriceValueTrait},
    steam_analyzer::analyze_steam_sell_history,
    storages::{
        CsfloatEngine, CsfloatEngineListingDecision, CsfloatEngineTrait, SteamEngine,
        SteamEngineTrait,
    },
    types::ListingId,
};

lazy_static! {
    static ref MARKET_HASH_NAME_REGEX: Regex =
        Regex::new(r#"<title>Steam Community Market :: Listings for (.+)</title>"#).unwrap();
}

fn extract_market_hash_name(input: &str) -> Option<String> {
    if let Some(captures) = MARKET_HASH_NAME_REGEX.captures(input) {
        if let Some(market_hash_name) = captures.get(1) {
            return Some(market_hash_name.as_str().to_string());
        }
    }

    None
}

pub async fn process_steam_response(
    steam_engine: &mut SteamEngine,
    event: &SteamResponseEvent,
) -> Vec<Event> {
    let market_name = extract_market_hash_name(&event.response);
    if market_name.is_none() {
        warn!("Failed to extract market_hash_name for {}", event.response);
        return vec![];
    }
    let market_name = market_name.unwrap();

    if let Some(res_uw) = analyze_steam_sell_history(&event.response, event.timestamp) {
        steam_engine.update(&market_name, res_uw);
    }

    vec![]
}

pub async fn process_updated_csfloat_listing(
    steam_engine: &mut SteamEngine,
    csfloat_engine: &mut CsfloatEngine,
    event: &UpdatedCsfloatListingsEvent,
) -> Vec<Event> {
    let mut result: Vec<Event> = vec![];

    for listing_id in event.listing_ids.iter() {
        let csfloat_item = csfloat_engine.hm.get(listing_id);
        if csfloat_item.is_none() {
            continue;
        }
        let csfloat_item = csfloat_item.unwrap();
        let market_name = &csfloat_item.item.market_hash_name;
        let steam_analysis = steam_engine.hm.get(market_name);
        if steam_analysis.is_none() {
            continue;
        }
        let steam_analysis = steam_analysis.unwrap();

        let steam_price = steam_analysis.get_price_by_percentile(DESIRED_PERCENTILE);
        if steam_price.is_none() {
            continue;
        }
        let steam_price = steam_price.unwrap();
        let csfloat_price = csfloat_item.get_price_value();
        let steam_no_fee = SteamFee::subtract_fee(steam_price);
        let sold_per_week = steam_analysis.sold_per_week.unwrap_or(0) as u64;
        let is_stable = steam_analysis.is_stable.unwrap_or(false);
        let profit_pct = ((steam_no_fee as f64 / csfloat_price as f64) - 1.0) * 100.0;
        if csfloat_price < steam_no_fee {
            result.push(Event::Secondary(SecEvent::ProfitableListing(
                ProfitableListingEvent {
                    kind: ProfitableListingKind::Profitable,
                    market_name: market_name.clone(),
                    listing_id: listing_id.clone(),
                    csfloat_price,
                    steam_price,
                    steam_no_fee,
                    sold_per_week,
                    is_stable,
                    profit_pct,
                    float: csfloat_item.item.float_value,
                },
            )));
        }
    }

    for listing_id in event.listing_ids.iter() {
        let csfloat_item = csfloat_engine.hm.get(listing_id);
        if csfloat_item.is_none() {
            continue;
        }

        let csfloat_item = csfloat_item.unwrap();
        if is_good_glock_phase_listing(csfloat_item) {
            let csfloat_price = csfloat_item.get_price_value();
            const EMPTY_PRICE: PriceValue = 0 as PriceValue;

            result.push(Event::Secondary(SecEvent::ProfitableListing(
                ProfitableListingEvent {
                    kind: ProfitableListingKind::GoodPhase,
                    market_name: csfloat_item.item.market_hash_name.clone(),
                    listing_id: listing_id.clone(),
                    csfloat_price,
                    steam_price: EMPTY_PRICE,
                    steam_no_fee: EMPTY_PRICE,
                    sold_per_week: 0,
                    is_stable: false,
                    profit_pct: 0.0,
                    float: csfloat_item.item.float_value,
                },
            )));
        }
    }

    result
}

pub async fn process_csfloat_one_listing_response(
    csfloat_engine: &mut CsfloatEngine,
    csfloat_scheduler: &mut CsfloatScheduler,
    event: &CsfloatOneListingResponseEvent,
) -> Vec<Event> {
    if event.timestamp.elapsed() > Duration::from_micros(100) {
        warn!(
            "Too big delay before CsfloatOneListingResponseEvent will be proccessed: {:?}",
            event.timestamp.elapsed()
        )
    }

    if let Ok(parsed_item) = serde_json::from_str::<CsfloatListingStruct>(&event.response) {
        return process_parsed_csfloat_listings(
            vec![parsed_item],
            csfloat_engine,
            csfloat_scheduler,
        );
    } else {
        // Handle the case when an item is malformed (e.g., print an error message)
        error!("Error parsing item");
    }

    vec![]
}

pub async fn process_csfloat_listings_response(
    csfloat_engine: &mut CsfloatEngine,
    csfloat_scheduler: &mut CsfloatScheduler,
    event: &CsfloatResponseEvent,
) -> Vec<Event> {
    if event.timestamp.elapsed() > Duration::from_micros(100) {
        warn!(
            "Too big delay before CsfloatResponseEvent will be proccessed: {:?}",
            event.timestamp.elapsed()
        )
    }

    if let Ok(parsed_items) = serde_json::from_str::<Vec<CsfloatListingStruct>>(&event.response) {
        return process_parsed_csfloat_listings(parsed_items, csfloat_engine, csfloat_scheduler);
    } else {
        // Handle the case when an item is malformed (e.g., print an error message)
        warn!("Error parsing item");
    }

    vec![]
}

fn process_parsed_csfloat_listings(
    parsed_items: Vec<CsfloatListingStruct>,
    csfloat_engine: &mut CsfloatEngine,
    csfloat_scheduler: &mut CsfloatScheduler,
) -> Vec<Event> {
    let listing_ids: Vec<ListingId> = parsed_items
        .iter()
        .filter(|listing| prefilter_listing(listing))
        .filter_map(|listing| match csfloat_engine.update_listing(listing) {
            CsfloatEngineListingDecision::New | CsfloatEngineListingDecision::Updated => {
                csfloat_scheduler.upsert_listing(&listing.id);
                assert_eq!(
                    csfloat_engine.get_size(),
                    csfloat_scheduler.get_size(),
                    "Something strange! Size of engine and scheduler is not equal!"
                );
                Some(listing.id.clone())
            }
            CsfloatEngineListingDecision::NotChanged => None,
            CsfloatEngineListingDecision::Removed => {
                csfloat_scheduler.remove_listing(&listing.id);
                assert_eq!(
                    csfloat_engine.get_size(),
                    csfloat_scheduler.get_size(),
                    "Something strange! Size of engine and scheduler is not equal!"
                );
                None
            }
        })
        .collect();

    match listing_ids.is_empty() {
        true => vec![],
        false => vec![Event::Primary(PrimEvent::UpdatedCsfloatListings(
            UpdatedCsfloatListingsEvent { listing_ids },
        ))],
    }
}

pub async fn process_profitable_listing(
    bot: &Bot,
    csfloat_autobuy: &mut CsfloatAutobuy,
    event: &ProfitableListingEvent,
) -> Vec<Event> {
    let text = format!(
        "Found item {:.2}% {} : ${} | steam minus fee ${} | steam ${} \n stable: {} \n sold per week: {} \n id: {} \n float: {:?} \n kind: {:?}",
        event.profit_pct,
        event.market_name,
        event.csfloat_price.to_usd(),
        event.steam_no_fee.to_usd(),
        event.steam_price.to_usd(),
        event.is_stable,
        event.sold_per_week,
        event.listing_id,
        event.float,
        event.kind,
    );

    if is_need_notify_via_telegram(event) {
        let bot_cloned = bot.clone();
        tokio::spawn(async move {
            let _ = bot_cloned
                .send_message(Recipient::Id(MY_TG_ID), text.clone())
                .await;
        });
    }

    if IS_AUTOBUY_ALLOWED && is_need_to_autobuy(event) {
        let listing_id = event.listing_id.to_string();
        let price = event.csfloat_price as PriceValue;
        let result = match csfloat_autobuy.buy_listing(&listing_id, price).await {
            Ok(is_success) => is_success,
            Err(err) => {
                warn!(
                    "Failed to buy listing_id {} for ${} because {:?}",
                    listing_id, price, err
                );
                false
            }
        };

        let bot_cloned = bot.clone();
        tokio::spawn(async move {
            let text = format!(
                "Tried to buy {} for ${}: {:?}",
                listing_id,
                price.to_usd(),
                result,
            );
            let _ = bot_cloned.send_message(Recipient::Id(MY_TG_ID), text).await;
        });
    }

    vec![]
}
