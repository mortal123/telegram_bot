mod data_api;
mod tokens;
mod types;
mod utils;

use chrono::DateTime;
use std::collections::{HashMap, HashSet};
use teloxide::{
    prelude::*,
    utils::{command::BotCommands, markdown::escape},
};
use tokens::save_tokens;
use types::{Metadata, TokenAmount, UserAction, UserActionContent};

use crate::{
    data_api::{account_transfers, Action},
    tokens::SOL_TOKEN,
};

struct TransactionSummary {
    balances: HashMap<String, i64>,
    timestamp: i64,
    interact_with: Vec<String>,
    hash: String,
}

impl TransactionSummary {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            timestamp: 0,
            interact_with: Vec::new(),
            hash: String::new(),
        }
    }

    pub fn balance_change(&mut self, token: String, amount: i64) {
        *self.balances.entry(token).or_insert(0) += amount;
    }

    pub fn set_timestamp(&mut self, timestamp: i64) {
        if self.timestamp == 0 {
            self.timestamp = timestamp
        } else if self.timestamp != timestamp {
            log::error!(
                "incorrect timestamp, expect {}, find {}",
                self.timestamp,
                timestamp,
            );
        }
    }

    pub fn interact_with(&mut self, addr: String) {
        self.interact_with.push(addr);
    }
}

async fn parse_transaction_summary(summary: &TransactionSummary) -> UserAction {
    let mut spends = Vec::new();
    let mut receives = Vec::new();

    for (k, &v) in summary.balances.iter() {
        if v > 0 {
            spends.push(TokenAmount::new_with_amount(k, v).await);
        } else {
            receives.push(TokenAmount::new_with_amount(k, -v).await);
        }
    }

    let metadata = Metadata {
        timestamp: summary.timestamp,
        transaction_hash: summary.hash.clone(),
    };
    if spends.is_empty() && receives.is_empty() {
        UserAction {
            content: types::UserActionContent::None,
            metadata,
        }
    } else if spends.is_empty() && receives.len() == 1 {
        UserAction {
            content: types::UserActionContent::Receive(receives[0].clone()),
            metadata,
        }
    } else if spends.len() == 1 && receives.is_empty() {
        UserAction {
            content: types::UserActionContent::Spend(spends[0].clone()),
            metadata,
        }
    } else if spends.len() == 1 && receives.len() == 1 {
        UserAction {
            content: types::UserActionContent::Exchange(types::Exchange {
                spend: spends[0].clone(),
                receive: receives[0].clone(),
                //todo
                involved_amm: HashSet::new(),
            }),
            metadata,
        }
    } else {
        UserAction {
            content: types::UserActionContent::Unknown,
            metadata,
        }
    }
}

// newest to oldest
async fn account_actions(user: String, from: i64, to: i64, limit: usize) -> Vec<UserAction> {
    log::debug!("account actions: from={}, to={}, limit={}", from, to, limit);
    let transactions = account_transfers(user.clone(), from, to, limit).await;

    let mut user_actions = Vec::new();
    for transaction in transactions {
        let mut summary = TransactionSummary::new();
        summary.hash = transaction.transaction_hash;

        for instruction in transaction.data {
            summary.set_timestamp(instruction.timestamp);

            if instruction.token == "" {
                continue;
            }

            if instruction.action == Action::Transfer
                || instruction.action == Action::TransferChecked
            {
                let source = instruction.source;
                let destination = if let Some(dest) = instruction.destination {
                    dest
                } else {
                    continue;
                };

                if source == user {
                    summary.interact_with(destination);
                    summary.balance_change(instruction.token, instruction.amount);
                } else if destination == user {
                    summary.interact_with(source);
                    summary.balance_change(instruction.token, -instruction.amount);
                }
            }
        }

        user_actions.push(parse_transaction_summary(&summary).await);
    }
    user_actions
}

async fn quiz(user: String, days: usize) -> String {
    let now = chrono::Utc::now();
    let from = now - chrono::Duration::try_days(days as i64).unwrap();

    let user_actions = account_actions(user, from.timestamp(), now.timestamp(), 0).await;

    let mut map: HashMap<String, (i64, i64, i64)> = HashMap::new();
    for user_action in user_actions.into_iter().rev() {
        if let UserActionContent::Exchange(ex) = user_action.content {
            if ex.spend.is_sol() && !ex.receive.is_sol() {
                // buy
                let (sol, other, _) = map.get(&ex.receive.address).unwrap_or(&(0i64, 0i64, 0));
                map.insert(
                    ex.receive.address,
                    (
                        *sol - ex.spend.amount as i64,
                        *other + ex.receive.amount as i64,
                        user_action.metadata.timestamp,
                    ),
                );
            } else if !ex.spend.is_sol() && ex.receive.is_sol() {
                // sell
                let (sol, other, _) = map.get(&ex.spend.address).unwrap_or(&(0i64, 0i64, 0));
                map.insert(
                    ex.spend.address,
                    (
                        *sol + ex.receive.amount as i64,
                        *other - ex.spend.amount as i64,
                        user_action.metadata.timestamp,
                    ),
                );
            }
        } else if let UserActionContent::Receive(tm) = user_action.content {
            if !tm.is_sol() {
                let (sol, other, _) = map.get(&tm.address).unwrap_or(&(0i64, 0i64, 0));
                map.insert(
                    tm.address,
                    (
                        *sol,
                        *other + tm.amount as i64,
                        user_action.metadata.timestamp,
                    ),
                );
            }
        }
    }

    let mut pairs: Vec<_> = map.into_iter().collect();
    pairs.sort_by_key(|(_, v)| v.2);

    let mut lines = Vec::new();
    for (k, v) in pairs.iter().rev() {
        let escaped_sol = escape(
            &TokenAmount::new_with_amount(&SOL_TOKEN.to_string(), v.0)
                .await
                .to_string(),
        );
        let escaped_other = escape(&TokenAmount::new_with_amount(k, v.1).await.to_string());
        let datetime = escape(&utils::datetime_to_string(
            DateTime::from_timestamp(v.2, 0).unwrap(),
        ));

        lines.push(format!(
            "[{}](https://solana.fm/address/{}): {} vs {}",
            datetime, k, escaped_sol, escaped_other,
        ));
    }
    lines.join("\n")
}

async fn actions(user: String, days: usize) -> String {
    let now = chrono::Utc::now();
    let from = now - chrono::Duration::try_days(days as i64).unwrap();

    let user_actions = account_actions(user, from.timestamp(), now.timestamp(), 0).await;
    user_actions
        .iter()
        // .filter(|a| match a.content {
        //     UserActionContent::None => false,
        //     _ => true,
        // })
        .take(50)
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Display this text.")]
    Help,
    #[command(
        description = "Show someone portfolio in the last x days, format like /quiz account days",
        parse_with = "split"
    )]
    Quiz { user: String, days: usize },
    #[command(
        description = "Show someone actions in the last few days. If more than 50 transaction in these days, at most 50 will be displayed",
        parse_with = "split"
    )]
    Actions { user: String, days: usize },
    #[command(description = "Save the tokens metadata into cache")]
    SaveTokens,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Quiz { user, days } => {
            let message = quiz(user, days).await;
            log::debug!("{}", message);
            bot.send_message(msg.chat.id, message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?
        }
        Command::Actions { user, days } => {
            let message = actions(user, days).await;
            log::debug!("{}", message);
            bot.send_message(msg.chat.id, message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?
        }
        Command::SaveTokens => {
            let size = save_tokens().await;
            bot.send_message(msg.chat.id, format!("Total {size} tokens are saved"))
                .await?
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    Command::repl(bot, answer).await;
}
