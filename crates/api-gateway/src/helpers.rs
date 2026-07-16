use market_analyzer::analyzer;
use crate::AppState;
use std::sync::Arc;

pub async fn get_active_pair(
    state: &AppState,
    pair_key: &str,
) -> Option<Arc<analyzer::ActivePair>> {
    state.get_active_pair(pair_key).await
}

/// Extract base symbol from a pair_key (e.g., "BTC-USDT" -> "BTC")
pub fn extract_base_symbol(pair_key: &str) -> String {
    pair_key.split('-').next().unwrap_or(pair_key).to_string()
}

/// Build pair_key from config symbol (e.g., config "Hyperliquid:BTC" or "BTC" -> "BTC-USDT")
pub fn default_pair_key(symbol_entry: &str) -> String {
    let raw = symbol_entry
        .split_once(':')
        .map(|(_, s)| s)
        .unwrap_or(symbol_entry);
    format!("{}-USDT", raw)
}
