use std::{collections::HashMap, fs::File, str::FromStr};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use mpl_token_metadata::accounts::Metadata;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Mint;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub address: String,
    pub decimals: usize,
    pub logo_uri: String,
}

pub static SOL_TOKEN: &'static str = "So11111111111111111111111111111111111111112";
pub static TOKEN_CACHE: &'static str = "./token-cache.json";

lazy_static::lazy_static! {
    // https://github.com/jup-ag/token-list/blob/main/src/partners/data/solana-fm.csv
    pub static ref TOKENS: Mutex<HashMap<String, Token>> = Mutex::new(load_tokens());

    pub static ref CLIENT: RpcClient = RpcClient::new(std::env::var("RPC_URL").unwrap());
}

pub async fn lookup_token(mint: &String) -> Token {
    let mut tokens = TOKENS.lock().await;
    if let Some(token) = tokens.get(mint) {
        return token.clone();
    }

    log::debug!("Lookup token on chain: {mint}");

    let mut token = Token {
        name: mint[..6].to_string(),
        symbol: "Unknown".to_string(),
        address: mint.clone(),
        decimals: 0,
        logo_uri: "".to_string(),
    };

    let mint_pubkey = Pubkey::from_str(mint).unwrap();

    match CLIENT.get_account_data(&mint_pubkey) {
        Ok(data) => {
            let data: Mint = Mint::unpack(&data).unwrap();
            log::debug!("Decimal: {}", data.decimals);
            token.decimals = data.decimals as usize;
        }
        Err(err) => {
            log::error!("Failed to fetch mint info for token: {mint}, err: {err}");
        }
    }

    let (metadata_pubkey, _) = Metadata::find_pda(&mint_pubkey);
    match CLIENT.get_account_data(&metadata_pubkey) {
        Ok(data) => {
            let metadata: Metadata = Metadata::from_bytes(&data).unwrap();
            log::debug!("Name: {}, Symbol: {}", metadata.name, metadata.symbol,);
            token.name = metadata.name.trim_end_matches('\u{0}').to_string();
            token.symbol = metadata.symbol.trim_end_matches('\u{0}').to_string();
        }
        Err(err) => {
            log::error!("Failed to fetch metadata for mint token: {mint}, err: {err}");
        }
    }

    tokens.insert(mint.to_string(), token.clone());

    token
}

pub fn load_tokens() -> HashMap<String, Token> {
    let file = File::open(TOKEN_CACHE).unwrap();
    let token_list: Vec<Token> = serde_json::from_reader(file).unwrap();
    log::info!("Loading {} tokens from cache", token_list.len());
    token_list
        .into_iter()
        .map(|t| (t.address.clone(), t))
        .collect()
}

pub async fn save_tokens() -> usize {
    let tokens = TOKENS.lock().await;

    let file = File::create(TOKEN_CACHE).unwrap();
    let mut token_list: Vec<_> = tokens.clone().into_values().collect();
    token_list.sort_by(|a, b| a.address.cmp(&b.address));
    log::info!("Saving {} tokens into cache", token_list.len());
    serde_json::to_writer_pretty(file, &token_list).unwrap();

    tokens.len()
}
