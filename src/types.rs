use std::{
    collections::HashSet,
    fmt::{self, Display, Formatter},
};

use chrono::DateTime;
use teloxide::utils::markdown::escape;

use crate::tokens::{SOL_TOKEN, TOKENS};

#[derive(Clone, Debug)]
pub struct TokenAmount {
    pub address: String,
    pub amount: i64,
}

impl TokenAmount {
    pub fn is_sol(&self) -> bool {
        self.address == SOL_TOKEN
    }

    pub fn short_address(&self) -> &str {
        &self.address[..6]
    }
}

impl Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match TOKENS.get(&self.address) {
            None => write!(f, "{} {}", self.amount, self.short_address()),
            Some(token) => {
                write!(
                    f,
                    "{} {}",
                    self.amount as f64 / 10f64.powf(token.decimals as f64),
                    token.symbol
                )
            }
        }
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
            datetime.format("%m/%d %H:%M:%S").to_string(),
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
