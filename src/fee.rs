use crate::prices::{PriceValue, PriceValueTrait};

pub struct SteamFee;

const WALLET_FEE_PERCENT: f64 = 0.05;
const DEFAULT_PUBLISHER_FEE: f64 = 0.1;
// 1 + WALLET_FEE_PERCENT + DEFAULT_PUBLISHER_FEE
const DIVIDER: f64 = 1.15;

impl SteamFee {
    #[inline]
    pub fn add_fee(payload: PriceValue) -> PriceValue {
        if payload < 1 {
            panic!("Unexpected input");
        }
        let steam_fee = payload.multiply_by_percent(WALLET_FEE_PERCENT).max(1);
        let game_fee = payload.multiply_by_percent(DEFAULT_PUBLISHER_FEE).max(1);

        payload + steam_fee + game_fee
    }

    #[inline]
    pub fn subtract_fee(total: PriceValue) -> PriceValue {
        if total < 3 {
            panic!("Unexpected input");
        }
        const MAX_STEPS: i32 = 4;
        const START_ADDITION_CENTS: u64 = 2;

        let predicted_payload = total.divide_by(DIVIDER);
        let mut payload = predicted_payload + START_ADDITION_CENTS;

        for _ in 0..MAX_STEPS {
            let calculated_total = SteamFee::add_fee(payload);
            if calculated_total <= total {
                break;
            }
            payload -= 1;
        }

        payload
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_is_strict_value() {
        assert_eq!(DIVIDER, 1.15);
    }
}
