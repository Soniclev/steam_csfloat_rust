use crate::{
    consts::{
        AUTOBUY_FROM_PROFIT_PCT, LISTING_MAX_PRICE, LISTING_MIN_PRICE, MIN_SOLD_PER_WEEK, PHASE_4,
        TG_NOTIFY_MIN_PROFIT_PCT,
    },
    events::{ProfitableListingEvent, ProfitableListingKind},
    models::CsfloatListingStruct,
    prices::PriceValue,
};

#[inline]
pub fn prefilter_listing(listing: &CsfloatListingStruct) -> bool {
    // true - listing is allowed
    // false - skip listing

    // Skip souvenir listings
    if listing.item.is_souvenir {
        return false;
    }

    // Skip too cheap or rich items
    if listing.price < LISTING_MIN_PRICE || listing.price > LISTING_MAX_PRICE {
        return false;
    }

    true
}

pub fn is_good_glock_phase_listing(listing: &CsfloatListingStruct) -> bool {
    if listing.item.phase.is_none() {
        return false;
    }

    let phase = listing.item.phase.as_ref().unwrap();

    if phase.as_str() == PHASE_4 {
        const FACTORY_NEW: &str = "Glock-18 | Gamma Doppler (Factory New)";
        const MINIMAL_WEAR: &str = "Glock-18 | Gamma Doppler (Minimal Wear)";
        const FIELD_TESTED: &str = "Glock-18 | Gamma Doppler (Field-Tested)";

        const FACTORY_NEW_PRICE: PriceValue = 60_00 as PriceValue; // $60
        const MINIMAL_WEAR_PRICE: PriceValue = 45_00 as PriceValue; // $45
        const FIELD_TESTED_PRICE: PriceValue = 35_00 as PriceValue; // $35

        let price = listing.price;

        return match listing.item.market_hash_name.as_str() {
            FACTORY_NEW => price <= FACTORY_NEW_PRICE,
            MINIMAL_WEAR => price <= MINIMAL_WEAR_PRICE,
            FIELD_TESTED => price <= FIELD_TESTED_PRICE,
            _ => false,
        };
    }

    false
}

pub fn is_need_notify_via_telegram(event: &ProfitableListingEvent) -> bool {
    if event.kind == ProfitableListingKind::GoodPhase {
        return true;
    }

    event.is_stable
        && event.sold_per_week >= MIN_SOLD_PER_WEEK
        && event.profit_pct > TG_NOTIFY_MIN_PROFIT_PCT
}

pub fn is_need_to_autobuy(event: &ProfitableListingEvent) -> bool {
    event.kind == ProfitableListingKind::Profitable && event.profit_pct > AUTOBUY_FROM_PROFIT_PCT
}
