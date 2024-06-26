use std::{
    collections::HashSet,
    fmt::{self, Display, Formatter},
};

use chrono::DateTime;
use teloxide::utils::markdown::escape;

use crate::{
    tokens::{lookup_token, SOL_TOKEN},
    utils::datetime_to_string,
};

#[derive(Clone, Debug)]
pub struct TokenAmount {
    pub address: String,
    pub decimals: usize,
    pub symbol: String,
    pub amount: i64,
}

impl TokenAmount {
    pub fn is_sol(&self) -> bool {
        self.address == SOL_TOKEN
    }

    pub async fn new(address: &String) -> Self {
        let token = lookup_token(address).await;
        Self {
            address: address.clone(),
            decimals: token.decimals,
            symbol: token.symbol,
            amount: 0,
        }
    }

    pub async fn new_with_amount(address: &String, amount: i64) -> Self {
        let mut s = Self::new(address).await;
        s.amount = amount;
        s
    }

    pub fn _amount_change(&mut self, change: i64) {
        self.amount += change;
    }
}

impl Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            self.amount as f64 / 10f64.powf(self.decimals as f64),
            self.symbol
        )
    }
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pub timestamp: i64,
    pub transaction_hash: String,
}

#[derive(Clone, Debug)]
pub struct UserAction {
    pub content: UserActionContent,
    pub metadata: Metadata,
}

impl Display for UserAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let datetime =
            DateTime::from_timestamp(self.metadata.timestamp, 0).expect("Invalid timestamp");
        let escaped_content = escape(&self.content.to_string());
        write!(
            f,
            "[{}](https://solana.fm/tx/{}): {}",
            datetime_to_string(datetime),
            self.metadata.transaction_hash,
            escaped_content,
        )
    }
}

#[derive(Clone, Debug)]
pub enum UserActionContent {
    Exchange(Exchange),
    Spend(TokenAmount),
    Receive(TokenAmount),
    None,
    Unknown,
}

impl Display for UserActionContent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UserActionContent::Exchange(exchange) => {
                write!(f, "Exchange {} with {}", exchange.spend, exchange.receive)
            }
            UserActionContent::Spend(token_amount) => write!(f, "Spend {}", token_amount),
            UserActionContent::Receive(token_amount) => write!(f, "Receive {}", token_amount),
            UserActionContent::None => write!(f, "None"),
            UserActionContent::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Exchange {
    pub spend: TokenAmount,
    pub receive: TokenAmount,
    pub involved_amm: HashSet<String>,
}
