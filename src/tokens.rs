use std::{collections::HashMap, fs::File};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

fn read_csv<R: DeserializeOwned>(file: File) -> Vec<R> {
    let mut reader = csv::Reader::from_reader(file);

    let mut vec = Vec::new();
    for result in reader.deserialize() {
        let record: R = result.unwrap();
        vec.push(record);
    }

    vec
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    #[serde(rename = "NAME")]
    pub name: String,
    #[serde(rename = "SYMBOL")]
    pub symbol: String,
    #[serde(rename = "ADDRESS")]
    pub address: String,
    #[serde(rename = "DECIMALS")]
    pub decimals: usize,
    #[serde(rename = "LOGOURI")]
    pub logo_uri: String,
    // #[serde(rename = "VERIFIED")]
    // pub verified: bool,
}

pub static SOL_TOKEN: &'static str = "So11111111111111111111111111111111111111112";

lazy_static::lazy_static! {
    // https://github.com/jup-ag/token-list/blob/main/src/partners/data/solana-fm.csv
    pub static ref TOKENS: HashMap<String, Token> = {
        let file = File::open("solana-fm.csv").unwrap();
        let tokens = read_csv::<Token>(file);

        let mut map = HashMap::new();
        for token in tokens {
            map.insert(token.address.clone(), token);
        }
        map
    };
}
