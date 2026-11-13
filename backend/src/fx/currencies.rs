//! Supported local currencies and money arithmetic.
//!
//! Fiat amounts are stored as integers in the currency's minor unit (e.g. cents) to
//! avoid floating-point rounding in records. Exchange rates are `f64` because they
//! are market quotes, and conversions round to the nearest whole unit.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Currency {
    pub code: &'static str,
    pub name: &'static str,
    /// Decimal places used for amounts (0 for e.g. UGX, XOF).
    pub decimals: u32,
    /// Example cash-out channels shown in the demo. Illustrative, not integrations.
    pub payout_providers: &'static [&'static str],
}

const fn c(
    code: &'static str,
    name: &'static str,
    decimals: u32,
    payout_providers: &'static [&'static str],
) -> Currency {
    Currency {
        code,
        name,
        decimals,
        payout_providers,
    }
}

/// Most African currencies, plus USD.
pub static CURRENCIES: &[Currency] = &[
    c(
        "KES",
        "Kenyan Shilling",
        2,
        &["M-Pesa", "Airtel Money", "Bank transfer"],
    ),
    c(
        "NGN",
        "Nigerian Naira",
        2,
        &["Bank transfer", "OPay", "PalmPay", "MoMo PSB"],
    ),
    c(
        "GHS",
        "Ghanaian Cedi",
        2,
        &["MTN MoMo", "Telecel Cash", "AT Money", "Bank transfer"],
    ),
    c("ZAR", "South African Rand", 2, &["Bank transfer (EFT)"]),
    c(
        "UGX",
        "Ugandan Shilling",
        0,
        &["MTN MoMo", "Airtel Money", "Bank transfer"],
    ),
    c(
        "TZS",
        "Tanzanian Shilling",
        2,
        &["M-Pesa", "Airtel Money", "Mixx by Yas", "Bank transfer"],
    ),
    c(
        "RWF",
        "Rwandan Franc",
        0,
        &["MTN MoMo", "Airtel Money", "Bank transfer"],
    ),
    c(
        "ETB",
        "Ethiopian Birr",
        2,
        &["telebirr", "M-Pesa", "Bank transfer"],
    ),
    c(
        "XOF",
        "West African CFA Franc",
        0,
        &["Orange Money", "Wave", "MTN MoMo", "Moov Money"],
    ),
    c(
        "XAF",
        "Central African CFA Franc",
        0,
        &["MTN MoMo", "Orange Money", "Bank transfer"],
    ),
    c(
        "EGP",
        "Egyptian Pound",
        2,
        &["Vodafone Cash", "InstaPay", "Bank transfer"],
    ),
    c("MAD", "Moroccan Dirham", 2, &["Bank transfer"]),
    c("DZD", "Algerian Dinar", 2, &["Bank transfer"]),
    c("TND", "Tunisian Dinar", 3, &["Bank transfer"]),
    c(
        "ZMW",
        "Zambian Kwacha",
        2,
        &["MTN MoMo", "Airtel Money", "Bank transfer"],
    ),
    c(
        "MWK",
        "Malawian Kwacha",
        2,
        &["Airtel Money", "TNM Mpamba", "Bank transfer"],
    ),
    c(
        "MZN",
        "Mozambican Metical",
        2,
        &["M-Pesa", "e-Mola", "Bank transfer"],
    ),
    c(
        "BWP",
        "Botswana Pula",
        2,
        &["Orange Money", "Bank transfer"],
    ),
    c("NAD", "Namibian Dollar", 2, &["Bank transfer"]),
    c(
        "CDF",
        "Congolese Franc",
        2,
        &["M-Pesa", "Orange Money", "Airtel Money"],
    ),
    c(
        "SLE",
        "Sierra Leonean Leone",
        2,
        &["Orange Money", "Africell Money"],
    ),
    c("GMD", "Gambian Dalasi", 2, &["Wave", "Bank transfer"]),
    c("LRD", "Liberian Dollar", 2, &["MTN MoMo", "Orange Money"]),
    c("USD", "US Dollar", 2, &["Bank transfer"]),
];

pub fn find(code: &str) -> Option<&'static Currency> {
    CURRENCIES.iter().find(|c| c.code == code)
}

/// Parses a user-entered amount like "1,500.50" into minor units.
pub fn parse_amount(input: &str, decimals: u32) -> Option<i64> {
    let cleaned: String = input
        .chars()
        .filter(|c| !matches!(c, ',' | ' ' | '_'))
        .collect();
    let (whole, frac) = match cleaned.split_once('.') {
        Some((w, f)) => (w, f),
        None => (cleaned.as_str(), ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
        || frac.len() > decimals as usize
        || whole.len() > 15
    {
        return None;
    }
    let scale = 10i64.checked_pow(decimals)?;
    let frac_padded = format!("{frac:0<width$}", width = decimals as usize);
    let frac_val: i64 = if decimals == 0 {
        0
    } else {
        frac_padded.parse().ok()?
    };
    whole
        .parse::<i64>()
        .ok()?
        .checked_mul(scale)?
        .checked_add(frac_val)
}

/// Formats minor units as a plain decimal string, e.g. 150050 (2 dp) -> "1500.50".
pub fn format_amount(minor: i64, decimals: u32) -> String {
    if decimals == 0 {
        return minor.to_string();
    }
    let scale = 10i64.pow(decimals);
    format!(
        "{}.{:0width$}",
        minor / scale,
        (minor % scale).abs(),
        width = decimals as usize
    )
}

/// sats for a fiat amount, given the price of 1 BTC in that currency.
pub fn sats_for_fiat(minor: i64, decimals: u32, btc_price: f64) -> i64 {
    let fiat = minor as f64 / 10f64.powi(decimals as i32);
    (fiat / btc_price * 100_000_000.0).round() as i64
}

/// Fiat minor units for an amount in sats.
pub fn fiat_for_sats(sats: i64, decimals: u32, btc_price: f64) -> i64 {
    (sats as f64 / 100_000_000.0 * btc_price * 10f64.powi(decimals as i32)).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_amounts_per_currency_precision() {
        assert_eq!(parse_amount("1,500.50", 2), Some(150_050));
        assert_eq!(parse_amount("1500", 2), Some(150_000));
        assert_eq!(parse_amount("1500.5", 2), Some(150_050));
        assert_eq!(parse_amount("2000", 0), Some(2000));
        assert_eq!(parse_amount("2000.5", 0), None, "UGX has no decimals");
        assert_eq!(parse_amount("1.2345", 3), None);
        for bad in ["", ".5", "-5", "1e5", "abc", "1.2.3", "9999999999999999"] {
            assert_eq!(parse_amount(bad, 2), None, "{bad}");
        }
    }

    #[test]
    fn formats_amounts() {
        assert_eq!(format_amount(150_050, 2), "1500.50");
        assert_eq!(format_amount(5, 2), "0.05");
        assert_eq!(format_amount(2000, 0), "2000");
        assert_eq!(format_amount(1234, 3), "1.234");
    }

    #[test]
    fn converts_both_ways() {
        // 1 BTC = 9,000,000 KES  ->  1 KES = 11.11 sats
        let price = 9_000_000.0;
        assert_eq!(sats_for_fiat(18_000, 2, price), 2000); // KES 180.00
        assert_eq!(fiat_for_sats(2000, 2, price), 18_000);
        // Zero-decimal currency.
        assert_eq!(sats_for_fiat(10_000, 0, 280_000_000.0), 3571);
    }

    #[test]
    fn currency_table_is_consistent() {
        for c in CURRENCIES {
            assert_eq!(c.code.len(), 3);
            assert!(c.code.bytes().all(|b| b.is_ascii_uppercase()));
            assert!(!c.payout_providers.is_empty());
            assert!(c.decimals <= 3);
        }
        // ISO 4217 minor units; the frontend's Intl data must agree.
        for (code, dec) in [
            ("KES", 2),
            ("TZS", 2),
            ("UGX", 0),
            ("RWF", 0),
            ("XOF", 0),
            ("XAF", 0),
            ("TND", 3),
        ] {
            assert_eq!(find(code).unwrap().decimals, dec, "{code}");
        }
        assert!(find("KES").is_some());
        assert!(find("XYZ").is_none());
        assert!(CURRENCIES.len() >= 20);
    }
}
