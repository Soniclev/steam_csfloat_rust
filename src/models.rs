use core::fmt;
use std::fmt::{Display, Formatter};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::prices::PriceValue;
use crate::types::MarketName;
use crate::utils::{naive_datetime_from_timestamp, naive_datetime_to_timestamp};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CsfloatListingState {
    Listed,
    Delisted,
    Sold,
    Refunded,
}

impl Display for CsfloatListingState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CsfloatListingItem {
    pub market_hash_name: MarketName,
    #[serde(default)]
    pub is_souvenir: bool,
    #[serde(default)]
    pub float_value: Option<f64>,
    #[serde(default)]
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CsfloatListingStruct {
    pub id: String,
    pub price: u64,
    pub state: CsfloatListingState,
    #[serde(
        deserialize_with = "naive_datetime_from_timestamp",
        serialize_with = "naive_datetime_to_timestamp"
    )]
    pub created_at: NaiveDateTime,
    pub item: CsfloatListingItem,
}

impl CsfloatListingStruct {
    pub fn get_price_value(&self) -> PriceValue {
        self.price as PriceValue
    }

    pub fn has_any_important_changes(&self, listing_struct: &CsfloatListingStruct) -> bool {
        if self.price != listing_struct.price {
            return true;
        }
        if self.state != listing_struct.state {
            return true;
        }
        false
    }
}
