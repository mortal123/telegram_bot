use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub message: String,
    pub results: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    pub data: Vec<Instruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub action: Action,

    pub status: String,

    pub source: String,
    #[serde(rename = "sourceAssociation")]
    pub source_association: Option<String>,

    pub destination: Option<String>,
    pub destination_association: Option<String>,

    pub token: String,

    pub amount: i64,

    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Action {
    #[serde(rename = "transfer")]
    Transfer,
    #[serde(rename = "transferChecked")]
    TransferChecked,
    #[serde(other)]
    Unknown,
}

const MAX_PAGE_NUMBER: usize = 100;

pub async fn account_transfers(user: String, from: i64, to: i64, limit: usize) -> Vec<Transaction> {
    let mut transactions = Vec::new();
    let mut page = 1;
    while transactions.len() < limit {
        let url = format!(
            "https://api.solana.fm/v0/accounts/{}/transfers?utcFrom={}&utcTo={}&page={}&limit={}",
            user, from, to, page, MAX_PAGE_NUMBER,
        );
        let response_text = reqwest::get(url).await.unwrap().text().await.unwrap();
        // fs::write("response.json", &response_text).unwrap();
        let response: Response = serde_json::from_str(&response_text).unwrap();
        let tx_num = response.results.len();

        transactions.extend(response.results);

        if tx_num < MAX_PAGE_NUMBER {
            break;
        }
        page += 1;
    }

    transactions
        .into_iter()
        .filter(|tx| {
            if tx.data.iter().any(|ins| ins.status != "Successful") {
                false
            } else {
                true
            }
        })
        .take(limit)
        .collect()
}
