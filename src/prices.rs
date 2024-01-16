pub type PriceValue = u64;

pub trait PriceValueTrait {
    fn from_usd_f64(value: f64) -> PriceValue;
    fn divide_by(&self, value: f64) -> PriceValue;
    fn multiply_by_percent(&self, percent: f64) -> PriceValue;
    fn to_usd(&self) -> f64;
}

impl PriceValueTrait for PriceValue {
    #[inline]
    fn to_usd(&self) -> f64 {
        // Assuming that PriceValue represents cents, so dividing by 100 to get USD
        let usd = *self as f64 / 100.0;
        // Rounding to two digits precision
        (usd * 100.0).round() / 100.0
    }

    #[inline]
    fn multiply_by_percent(&self, percent: f64) -> PriceValue {
        (*self as f64 * percent) as PriceValue
    }

    #[inline]
    fn divide_by(&self, value: f64) -> PriceValue {
        (*self as f64 / value) as PriceValue
    }

    #[inline]
    fn from_usd_f64(value: f64) -> PriceValue {
        (value * 100.0) as PriceValue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_usd() {
        let price_value: PriceValue = 1000; // $10.00
        assert_eq!(price_value.to_usd(), 10.00);
    }

    #[test]
    fn test_multiply_by_percent() {
        let price_value: PriceValue = 1000; // $10.00
        let multiplied_value = price_value.multiply_by_percent(0.5); // 50%
        assert_eq!(multiplied_value, 500); // $5.00
    }

    #[test]
    fn test_divide_by() {
        let price_value: PriceValue = 1000; // $10.00
        let divided_value = price_value.divide_by(2.0); // Divide by 2
        assert_eq!(divided_value, 500); // $5.00
    }

    #[test]
    fn test_from_usd_f64() {
        let usd_value = 10.0; // $10.00
        let price_value = PriceValue::from_usd_f64(usd_value);
        assert_eq!(price_value, 1000); // $10.00
    }
}
