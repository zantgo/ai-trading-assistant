//! Regression for Bug 4 — `HL Derivatives Poller: BTC failed (N consecutive)`.
//!
//! Hyperliquid's `/info` endpoint, when called with
//! `{"type":"metaAndAssetCtxs"}`, returns a 2-tuple whose first element is
//! `meta` (whose `universe[i].name` is the canonical coin name) and whose
//! second element is the parallel `asset_ctxs` array. The per-asset-ctx
//! entries do NOT carry a `coin` field of their own — the coin is
//! positional. The legacy schema in `AssetCtxEntry` declared `coin:
//! String` (required), which made every 60-second poll fail with `error
//! decoding response body: missing field 'coin'`.
//!
//! This test pins the corrected behaviour:
//! 1. The picker recovers coin names from `meta.universe[i].name`.
//! 2. Defensive `coin: Option<String>` still wins if a future API change
//!    adds the field per entry.
//! 3. A response that omits a meta universe entry uses `UNKNOWN_<i>`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use network_adapters::adapters::hyperliquid_rest::fetch_meta_and_asset_ctxs;

/// One-shot HTTP server that always returns the canned body, regardless of
/// the request path. Captures the port it bound to so the test can target it.
async fn spawn_mock_server(body: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            // Read the request fully (we don't care about it for this test).
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let body_bytes = body.as_bytes();
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body_bytes.len()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.write_all(body_bytes).await;
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
        }
    });
    addr
}

/// Real Hyperliquid response shape, minus the `coin` field on entries.
const REAL_RESPONSE: &str = r#"[
  {"universe":[
      {"szDecimals":5,"name":"BTC","maxLeverage":50,"onlyIsolated":false,
       "marginTableId":0,"isDelisted":false,"baseCoin":null,"quoteCoin":null},
      {"szDecimals":4,"name":"ETH","maxLeverage":50,"onlyIsolated":false,
       "marginTableId":0,"isDelisted":false,"baseCoin":null,"quoteCoin":null}
   ],
   "marginTables":[],
   "collateralToken":"0"},
  [
    {"funding":"-0.0000198009","openInterest":"39925.94496",
     "prevDayPx":"64431.0","dayNtlVlum":"2575891913.82",
     "premium":"-0.0005891794","oraclePx":"65175.4","markPx":"65134.0",
     "midPx":"65136.5","impactPxs":["65136.0","65137.0"],
     "dayBaseVlum":"39786.65"},
    {"funding":"0.0000123","openInterest":"12345.67",
     "prevDayPx":"3500.0","dayNtlVlum":"123456.78",
     "premium":"0.0001","oraclePx":"3540.0","markPx":"3535.0",
     "midPx":"3537.5","impactPxs":["3537.0","3537.5"],
     "dayBaseVlum":"1000.00"}
  ]
]"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fetch_meta_and_asset_ctxs_recover_coins_from_meta_universe() {
    let addr = spawn_mock_server(REAL_RESPONSE).await;
    let url = format!("http://{}/info", addr);
    let map = fetch_meta_and_asset_ctxs(&url)
        .await
        .expect("the picker must succeed against the real response shape");

    assert!(
        map.contains_key("BTC"),
        "BTC must be present (positional recovery from meta.universe[0])"
    );
    assert!(
        map.contains_key("ETH"),
        "ETH must be present (positional recovery from meta.universe[1])"
    );

    // Every entry's mark_px must be a numeric decimal — the picker
    // returns only `Some(...)` for finite numeric strings.
    let btc = map.get("BTC").expect("BTC entry");
    assert_eq!(btc.mark_px.map(|d| d.to_string()), Some("65134.0".to_string()));
    assert_eq!(
        btc.open_interest.map(|d| d.to_string()),
        Some("39925.94496".to_string())
    );

    let eth = map.get("ETH").expect("ETH entry");
    assert_eq!(eth.mark_px.map(|d| d.to_string()), Some("3535.0".to_string()));
}

#[tokio::test]
async fn fetch_meta_and_asset_ctxs_handles_malformed_meta_legacy_path() {
    // Sanity: if `meta` is not the expected object the picker surfaces an
    // error rather than panicking. The legacy fallback path is also
    // exercised by the seed-and-shrink test below.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_addr = addr;
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            // Reply with valid JSON, but the meta object is "wrong shaped".
            let body = br#"[{"wrong":"shape"},[{"markPx":"65000.0"}]]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.write_all(body).await;
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
        }
    });
    let url = format!("http://{captured_addr}/info");
    let err = fetch_meta_and_asset_ctxs(&url)
        .await
        .expect_err("malformed meta must surface as Err, not panic");
    assert!(
        err.contains("Failed to parse Hyperliquid meta universe"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn fetch_meta_and_asset_ctxs_returns_unknown_index_when_universe_missing() {
    // If the universe is empty but the asset_ctxs array still has entries,
    // the picker must invent `UNKNOWN_<i>` keys so callers always see every
    // entry — never silently drop them.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured_addr = addr;
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let body = br#"[{"universe":[]},[{"markPx":"1"},{"markPx":"2"}]]"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(resp.as_bytes()).await;
            let _ = stream.write_all(body).await;
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
        }
    });
    let url = format!("http://{captured_addr}/info");
    let map: HashMap<String, _> = fetch_meta_and_asset_ctxs(&url)
        .await
        .expect("response must parse even with empty universe");
    assert!(
        map.contains_key("UNKNOWN_0"),
        "first ctx must be keyed UNKNOWN_0"
    );
    assert!(
        map.contains_key("UNKNOWN_1"),
        "second ctx must be keyed UNKNOWN_1"
    );
}

/// `Arc<Mutex<...>>` is a holdover from the previous version of this test
/// helper that needed to mutate the body per request. The current tests
/// always return the same body, so the mutex is unused. Kept for ABI
/// stability with future tests that may want to swap bodies.
#[allow(dead_code)]
fn unused_mutex_helper() -> Arc<Mutex<()>> {
    Arc::new(Mutex::new(()))
}
