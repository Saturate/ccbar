use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

const MAX_AGE_SECS: u64 = 24 * 60 * 60;
const API_URL: &str = "https://api.frankfurter.dev/v1/latest?from=USD";

#[derive(Deserialize)]
struct ApiResponse {
    rates: HashMap<String, f64>,
}

#[derive(serde::Serialize, Deserialize)]
struct CachedRates {
    fetched_at: u64,
    rates: HashMap<String, f64>,
}

fn cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(home).join(".cache"))
        .join("ccbar")
        .join("rates.json")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_cached() -> Option<CachedRates> {
    let content = fs::read_to_string(cache_path()).ok()?;
    let cached: CachedRates = serde_json::from_str(&content).ok()?;
    if now_secs() - cached.fetched_at < MAX_AGE_SECS {
        Some(cached)
    } else {
        None
    }
}

fn fetch_and_cache() -> Option<CachedRates> {
    let body: String = ureq::get(API_URL).call().ok()?.body_mut().read_to_string().ok()?;
    let resp: ApiResponse = serde_json::from_str(&body).ok()?;
    let cached = CachedRates {
        fetched_at: now_secs(),
        rates: resp.rates,
    };
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let json = serde_json::to_string(&cached).ok()?;
    let _ = fs::write(&path, json);
    Some(cached)
}

pub fn get_rate(currency: &str) -> Option<f64> {
    let upper = currency.to_uppercase();
    if upper == "USD" {
        return Some(1.0);
    }
    let cached = load_cached().or_else(fetch_and_cache)?;
    cached.rates.get(&upper).copied()
}

pub fn symbol(currency: &str) -> &str {
    match currency.to_uppercase().as_str() {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "JPY" | "CNY" => "¥",
        "KRW" => "₩",
        "INR" => "₹",
        "BRL" => "R$",
        "TRY" => "₺",
        "THB" => "฿",
        "CHF" => "CHF ",
        "DKK" | "SEK" | "NOK" | "ISK" | "CZK" => "kr ",
        "PLN" => "zł",
        "ILS" => "₪",
        "ZAR" => "R",
        "MXN" | "ARS" | "CLP" | "COP" => "$",
        "AUD" | "CAD" | "NZD" | "SGD" | "HKD" => "$",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usd_rate_is_identity() {
        assert_eq!(get_rate("USD"), Some(1.0));
        assert_eq!(get_rate("usd"), Some(1.0));
    }

    #[test]
    fn known_symbols() {
        assert_eq!(symbol("USD"), "$");
        assert_eq!(symbol("EUR"), "€");
        assert_eq!(symbol("GBP"), "£");
        assert_eq!(symbol("JPY"), "¥");
        assert_eq!(symbol("DKK"), "kr ");
        assert_eq!(symbol("SEK"), "kr ");
        assert_eq!(symbol("NOK"), "kr ");
        assert_eq!(symbol("CHF"), "CHF ");
        assert_eq!(symbol("INR"), "₹");
        assert_eq!(symbol("BRL"), "R$");
        assert_eq!(symbol("AUD"), "$");
        assert_eq!(symbol("CAD"), "$");
    }

    #[test]
    fn unknown_currency_returns_empty_symbol() {
        assert_eq!(symbol("XYZ"), "");
    }

    #[test]
    fn symbol_is_case_insensitive() {
        assert_eq!(symbol("eur"), "€");
        assert_eq!(symbol("Gbp"), "£");
        assert_eq!(symbol("dkk"), "kr ");
    }

    #[test]
    fn cached_rates_round_trip() {
        let mut rates = HashMap::new();
        rates.insert("DKK".to_string(), 6.85);
        rates.insert("EUR".to_string(), 0.85);
        let cached = CachedRates {
            fetched_at: 1000000,
            rates,
        };
        let json = serde_json::to_string(&cached).unwrap();
        let parsed: CachedRates = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.fetched_at, 1000000);
        assert_eq!(parsed.rates["DKK"], 6.85);
        assert_eq!(parsed.rates["EUR"], 0.85);
    }
}
