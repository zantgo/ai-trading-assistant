use std::sync::atomic::AtomicBool;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum Currency {
    USDT,
    USDC,
}

impl Currency {
    pub fn as_str(&self) -> &'static str {
        match self {
            Currency::USDT => "USDT",
            Currency::USDC => "USDC",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExchangeChoice {
    Hyperliquid,
    Bitget,
}

impl ExchangeChoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExchangeChoice::Hyperliquid => "Hyperliquid",
            ExchangeChoice::Bitget => "Bitget",
        }
    }

    pub fn raw_symbol(&self, base: &str, quote: &Currency) -> String {
        match self {
            ExchangeChoice::Hyperliquid => base.to_string(),
            ExchangeChoice::Bitget => match quote {
                Currency::USDT => format!("{}USDT", base),
                Currency::USDC => format!("{}USD", base),
            },
        }
    }

    pub fn internal_symbol(&self, base: &str, quote: &Currency) -> String {
        format!("{}-{}", base, quote.as_str())
    }

    pub fn bitget_product_type(&self, quote: &Currency) -> Option<&'static str> {
        match self {
            ExchangeChoice::Bitget => Some(match quote {
                Currency::USDT => "USDT-FUTURES",
                Currency::USDC => "USDC-FUTURES",
            }),
            ExchangeChoice::Hyperliquid => None,
        }
    }

    pub fn supports_currency(&self, quote: &Currency) -> bool {
        match self {
            ExchangeChoice::Hyperliquid => *quote == Currency::USDC,
            ExchangeChoice::Bitget => matches!(quote, Currency::USDT | Currency::USDC),
        }
    }
}

pub struct SessionState {
    pub active: AtomicBool,
    pub base_currency: RwLock<Option<Currency>>,
    pub exchange: RwLock<Option<ExchangeChoice>>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            base_currency: RwLock::new(None),
            exchange: RwLock::new(None),
        }
    }
}
