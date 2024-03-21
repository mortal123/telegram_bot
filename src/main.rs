mod data_api;
mod tokens;
mod types;

use std::collections::{HashMap, HashSet};
use teloxide::{
    prelude::*,
    utils::{command::BotCommands, markdown::escape},
};
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

fn parse_transaction_summary(summary: &TransactionSummary) -> UserAction {
    let spends: Vec<_> = summary
        .balances
        .iter()
        .filter(|(_, &v)| v > 0)
        .map(|(k, v)| TokenAmount {
            address: k.clone(),
            amount: *v,
        })
        .collect();

    let receives: Vec<_> = summary
        .balances
        .iter()
        .filter(|(_, &v)| v < 0)
        .map(|(k, v)| TokenAmount {
            address: k.clone(),
            amount: -*v,
        })
        .collect();

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

async fn account_actions(user: String, from: i64, to: i64) -> Vec<UserAction> {
    let transactions = account_transfers(user.clone(), from, to).await;

    let mut user_actions = Vec::new();
    for transaction in transactions {
        let mut summary = TransactionSummary::new();
        summary.hash = transaction.transaction_hash;

        for instruction in transaction.data {
            if instruction.token == "" {
                continue;
            }

            summary.set_timestamp(instruction.timestamp);

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

        user_actions.push(parse_transaction_summary(&summary));
    }
    user_actions
}

async fn quiz(user: String, days: usize) -> String {
    // AMM
    // let amms = HashMap::from([
    //     (
    //         "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1".to_string(),
    //         "RAYDIUM_V4".to_string(),
    //     ),
    //     (
    //         "BQ72nSv9f3PRyRKCBnHLVrerrv37CYTHm5h3s9VSGQDV".to_string(),
    //         "JUPITER_V6".to_string(),
    //     ),
    // ]);

    // 山哥notion上的 copy trader
    // let user = "HfcB5GVWnUvLsNGeGo9CZEYqVmy8QViZBRtRo4mjJeBe".to_string();
    // let user = "Cu5VRDQDnxSmSLUuRc2znNnxoCKJM9VEbXUTUKwUcHk9".to_string();
    let now = chrono::Utc::now();
    let from = now - chrono::Duration::try_days(days as i64).unwrap();

    let user_actions = account_actions(user, from.timestamp(), now.timestamp()).await;

    let mut map: HashMap<String, (i64, i64)> = HashMap::new();
    for user_action in user_actions {
        if let UserActionContent::Exchange(ex) = user_action.content {
            if ex.spend.is_sol() && !ex.receive.is_sol() {
                // buy
                let (sol, other) = map.get(&ex.receive.address).unwrap_or(&(0i64, 0i64));
                map.insert(
                    ex.receive.address,
                    (
                        *sol - ex.spend.amount as i64,
                        *other + ex.receive.amount as i64,
                    ),
                );
            } else if !ex.spend.is_sol() && ex.receive.is_sol() {
                // sell
                let (sol, other) = map.get(&ex.spend.address).unwrap_or(&(0i64, 0i64));
                map.insert(
                    ex.spend.address,
                    (
                        *sol + ex.receive.amount as i64,
                        *other - ex.spend.amount as i64,
                    ),
                );
            }
        } else if let UserActionContent::Receive(tm) = user_action.content {
            if !tm.is_sol() {
                let (sol, other) = map.get(&tm.address).unwrap_or(&(0i64, 0i64));
                map.insert(tm.address, (*sol, *other + tm.amount as i64));
            }
        }
    }
    map.iter()
        .map(|(k, v)| {
            let escaped_sol = escape(
                &TokenAmount {
                    address: SOL_TOKEN.to_string(),
                    amount: v.0,
                }
                .to_string(),
            );
            let escaped_other = escape(
                &TokenAmount {
                    address: k.clone(),
                    amount: v.1,
                }
                .to_string(),
            );
            format!(
                "[{}](https://solana.fm/address/{}): {} vs {}",
                &k[..6],
                k,
                escaped_sol,
                escaped_other
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn actions(user: String, num: usize) -> String {
    let now = chrono::Utc::now();
    let from = now - chrono::Duration::try_days(7).unwrap();

    let user_actions = account_actions(user, from.timestamp(), now.timestamp()).await;
    user_actions
        .iter()
        .take(num)
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
        description = "Show someone last 'number' of actions in the last few days.",
        parse_with = "split"
    )]
    Actions { user: String, num: usize },
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
        Command::Actions { user, num } => {
            let message = actions(user, num).await;
            log::debug!("{}", message);
            bot.send_message(msg.chat.id, message)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
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
