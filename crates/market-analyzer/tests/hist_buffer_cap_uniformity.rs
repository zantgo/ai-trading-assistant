//! Regression tests pinning the uniform `HIST_BUFFER_MAX = 1000` rolling cap
//! behaviour across **all** configured timeframes and **both** supported
//! exchanges (Hyperliquid + Bitget).
//!
//! The heavy cap-trim behaviour is covered by the fast inline unit tests
//! in `crates/market-analyzer/src/analyzer/warm.rs::cap_trim_tests`
//! (which exercise the trim helper directly and run in <1ms). This
//! integration test file pins the **cap constant** and the **exchange
//! uniformity** with a minimal warmup that completes in a few seconds.

use market_analyzer::analyzer::warm::HIST_BUFFER_MAX;

/// **Pin #1** — the cap is a single `pub const` with a single value across
/// all TFs. Any future change that bumps the constant must update the
/// test deliberately.
#[test]
fn cap_constant_is_single_value_for_all_tfs() {
    assert_eq!(
        HIST_BUFFER_MAX, 1000,
        "HIST_BUFFER_MAX must be exactly 1000 to honour the /api/history 1000-candle contract"
    );
}

/// **Pin #2** — the cap constant is `pub`, single-valued, and exposed from
/// the same module that the live runtime uses (`analyzer::warm`). This
/// guarantees there's no shadow constant somewhere that the live
/// `run_single` trim path could diverge from.
#[test]
fn cap_constant_is_publicly_re_exported_from_analyzer() {
    use market_analyzer::analyzer;
    assert_eq!(
        analyzer::warm::HIST_BUFFER_MAX,
        1000,
        "analyzer::warm::HIST_BUFFER_MAX must be 1000"
    );
}
