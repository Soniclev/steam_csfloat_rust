use std::time::Instant;

use chrono::{DateTime, NaiveDate, Utc};

use crate::{
    csfloat::CsfloatScheduler,
    event_processors::{process_csfloat_one_listing_response, process_steam_response},
    events::{
        CsfloatOneListingResponseEvent, Event, PrimEvent, SteamResponseEvent,
        UpdatedCsfloatListingsEvent,
    },
    models::CsfloatListingState,
    prices::PriceValue,
    storages::{CsfloatEngine, CsfloatEngineTrait, SteamEngine},
    types::ListingId,
};

#[tokio::test]
async fn test_process_steam_response() {
    let mut steam_engine = SteamEngine::new();
    let input = std::fs::read_to_string("src/test_data/Kilowatt Case.html")
        .expect("Failed to read HTML content from file");

    let faked_datetime = NaiveDate::from_ymd_opt(2024, 02, 19)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let event = SteamResponseEvent {
        response: input,
        timestamp: DateTime::from_naive_utc_and_offset(faked_datetime, Utc),
    };

    let result = process_steam_response(&mut steam_engine, &event).await;
    let analysis_result = steam_engine.hm.get("Kilowatt Case").unwrap();

    assert_eq!(analysis_result.is_stable, Some(false));
    assert_eq!(analysis_result.sold_per_week, Some(604_240));
    assert_eq!(analysis_result.rsd, Some(0.04770835480294064));

    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_process_csfloat_one_listing_response() {
    // Prepare your test data
    let mut csfloat_engine = CsfloatEngine::new(); // Initialize your CsfloatEngine
    let mut csfloat_scheduler = CsfloatScheduler::new(); // Initialize your CsfloatScheduler
    let response = r#"
        {
            "id": "679718648830624407",
            "created_at": "2024-02-19T15:59:14.443752Z",
            "type": "buy_now",
            "price": 355,
            "state": "listed",
            "seller": {
                "avatar": "https://avatars.steamstatic.com/d68a1e6123ba67b6d279e7bd6cfa86cb8acfed4b_full.jpg",
                "away": false,
                "flags": 48,
                "has_valid_steam_api_key": true,
                "online": true,
                "stall_public": true,
                "statistics": {
                    "median_trade_time": 3893,
                    "total_avoided_trades": 8,
                    "total_failed_trades": 16,
                    "total_trades": 1507,
                    "total_verified_trades": 1491
                },
                "steam_id": "76561198312080885",
                "username": "!Ca'St ⇆ TRADING",
                "verification_mode": "key"
            },
            "reference": {
                "base_price": 347,
                "float_factor": 1.0372812,
                "predicted_price": 360,
                "quantity": 612,
                "last_updated": "2024-02-19T15:59:14.438914Z"
            },
            "item": {
                "asset_id": "35764903495",
                "def_index": 4,
                "paint_index": 586,
                "paint_seed": 922,
                "float_value": 0.13217909634113312,
                "icon_url": "-9a81dlWLwJ2UUGcVs_nsVtzdOEdtWwKGZZLQHTxDZ7I56KU0Zwwo4NUX4oFJZEHLbXH5ApeO4YmlhxYQknCRvCo04DEVlxkKgposbaqKAxf0Ob3djFN79eJg4GYg_L4MrXVqXlU6sB9teXI8oThxlaxqhE_ZGj6I9OccFQ3YwmE-1C5x-u61sC0tM7JwSAy6ydx4XqOnxepwUYbufdxgq4",
                "d_param": "2344242939710818286",
                "is_stattrak": false,
                "is_souvenir": false,
                "rarity": 6,
                "quality": 4,
                "market_hash_name": "Glock-18 | Wasteland Rebel (Minimal Wear)",
                "tradable": 0,
                "inspect_link": "steam://rungame/730/76561202255233023/+csgo_econ_action_preview%20S76561198312080885A35764903495D2344242939710818286",
                "has_screenshot": false,
                "is_commodity": false,
                "type": "skin",
                "rarity_name": "Covert",
                "type_name": "Skin",
                "item_name": "Glock-18 | Wasteland Rebel",
                "wear_name": "Minimal Wear",
                "description": "It has been distressed, block printed, and painted with graffiti.\\n\\n\u003ci\u003ePay tribute\u003c/i\u003e",
                "collection": "The Gamma Collection"
            },
            "is_seller": false,
            "is_watchlisted": false,
            "watchers": 0
        }
    "#;
    let event = CsfloatOneListingResponseEvent {
        timestamp: Instant::now(), // Set the timestamp to the current time
        response: response.to_string(), // Provide your test JSON response
                                   // You might need to provide other fields if they're required by your implementation
    };

    // Call the function being tested
    let result =
        process_csfloat_one_listing_response(&mut csfloat_engine, &mut csfloat_scheduler, &event)
            .await;

    let listing_id: ListingId = "679718648830624407".to_string();

    // Assert the result against your expectations
    assert_eq!(result.len(), 1);
    let produced_event = result.get(0).unwrap();
    assert_eq!(
        *produced_event,
        Event::Primary(PrimEvent::UpdatedCsfloatListings(
            UpdatedCsfloatListingsEvent {
                listing_ids: vec![listing_id.clone()]
            },
        ))
    );

    assert_eq!(csfloat_engine.get_size(), 1);
    let stored_item = csfloat_engine.hm.get(&listing_id).unwrap();
    assert_eq!(stored_item.id, listing_id);
    assert_eq!(stored_item.price, 355 as PriceValue);
    assert_eq!(stored_item.state, CsfloatListingState::Listed);
    assert_eq!(stored_item.item.float_value, Some(0.13217909634113312));
    assert_eq!(stored_item.item.is_souvenir, false);
    assert_eq!(
        stored_item.item.market_hash_name,
        "Glock-18 | Wasteland Rebel (Minimal Wear)".to_string()
    );
}
