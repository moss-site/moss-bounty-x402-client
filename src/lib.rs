//! [![Crates.io](https://img.shields.io/crates/v/moss-bounty-x402-client)](https://crates.io/crates/moss-bounty-x402-client)
//! [![docs](https://img.shields.io/crates/v/moss-bounty-x402-client?color=yellow&label=docs)](https://docs.rs/moss-bounty-x402-client)
//!
//! Moss Bounty X402 Client
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! moss-bounty-x402-client = "0.1.0"
//! ```
//!
//! ## Basic Usage
//!
//! ```rust
//! use moss-bounty-x402-client::{Client, CreateBountyTaskData};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // User moss authorization token, fetch by login
//!     let auth_token = "<Your-Moss-Authorization-Token>";
//!
//!     // User wallet private key for bounty payment
//!     let private = "<Your-Wallet-Private-Key>";
//!
//!     // Build a client
//!     let client = Client::new(&auth_token, &private)?;
//!
//!     // Build task data
//!     let task = CreateBountyTaskData {
//!         target_twitter_handle: "<target_twitter_handle>".to_string(),
//!         question: "Hello?".to_string(),
//!         amount_usdc: "1000000".to_string(),
//!         valid_hours: 12,
//!     }
//!
//!     // Create bounty task
//!     client.create_bounty_task(task)?
//!
//!     Ok(())
//! }
//! ```

mod eip3009;
use alloy::primitives::{Address, B256, U256};
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateBountyTaskData {
    pub target_twitter_handle: String,
    pub question: String,
    pub amount_usdc: String,
    pub valid_hours: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBounty402Resp {
    accepts: Vec<X402Accept>,
    x402_version: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct X402Accept {
    schema: String,
    network: String,
    max_amount_required: String,
    pay_to: String,
    max_time_seconds: i64,
    asset: String,
    extra: X402AcceptExtra,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct X402AcceptExtra {
    name: String,
    version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct X402Payment {
    x402_version: usize,
    scheme: String,
    network: String,
    payload: X402PaymentPayload,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct X402PaymentPayload {
    signature: String,
    authorization: X402Authorization,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct X402Authorization {
    mime_type: String,
    from: String,
    to: String,
    value: String,
    valid_after: String,
    valid_before: String,
    nonce: String,
}

const MOSS_API_HOST: &str = "https://moss-dev.moss.site";

pub struct Client {
    auth_token: String,
    host: String,
    signer: PrivateKeySigner,
}

impl Client {
    pub fn new(auth_token: &str, wallet_key: &str) -> Result<Self> {
        Ok(Client {
            host: MOSS_API_HOST.to_string(),
            auth_token: auth_token.to_string(),
            signer: wallet_key.parse::<PrivateKeySigner>()?,
        })
    }

    pub async fn create_bounty_task(&self, data: CreateBountyTaskData) -> Result<()> {
        const API_PATH: &str = "/api/v1/bounty/tasks";
        let url = format!("{}/{}", self.host, API_PATH);

        let rsp = reqwest::Client::default()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        if rsp.status() != 402 {
            return Err(anyhow::anyhow!(
                "Error creating bounty task: {}",
                rsp.status()
            ));
        }

        let rsp = rsp.json::<CreateBounty402Resp>().await?;

        let payment = self.build_x402_payment(rsp)?;
        let payment = general_purpose::STANDARD.encode(serde_json::to_vec(&payment)?.as_slice());

        let rsp = reqwest::Client::default()
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.auth_token))
            .header("X-Payment", payment)
            .header("Content-Type", "application/json")
            .json(&data)
            .send()
            .await?;

        if !rsp.status().is_success() {
            return Err(anyhow::anyhow!("Failed request {}", rsp.status()));
        }

        Ok(())
    }

    fn build_x402_payment(&self, rsp: CreateBounty402Resp) -> Result<X402Payment> {
        if rsp.x402_version != 1 {
            return Err(anyhow::anyhow!("Unsupported x402 version"));
        }

        if rsp.accepts.len() < 1 {
            return Err(anyhow::anyhow!("No payment method found"));
        }

        let accept = &rsp.accepts[0];

        if accept.schema != "exact" || accept.network != "base" {
            return Err(anyhow::anyhow!(
                "Error creating bounty task: {}",
                accept.schema
            ));
        }

        let domain = eip3009::Domain {
            name: accept.extra.name.clone(),
            version: accept.extra.version.clone(),
            chain_id: eip3009::BASE_CHAIN_ID,
            verifying_contract: Address::from_str(&accept.asset)?,
        };

        let message = eip3009::Message {
            from: self.signer.address(),
            to: accept.pay_to.parse::<Address>()?,
            value: accept.max_amount_required.parse::<U256>()?,
            valid_after: U256::from(0),
            valid_before: U256::from(chrono::Utc::now().timestamp() + accept.max_time_seconds),
            nonce: generate_nonce(),
        };

        let signature = eip3009::signing_hash(domain, &message)?;

        let authorization = X402Authorization {
            mime_type: "application/json".to_string(),
            from: message.from.to_string(),
            to: message.to.to_string(),
            value: message.value.to_string(),
            valid_after: message.valid_after.to_string(),
            valid_before: message.valid_before.to_string(),
            nonce: message.nonce.to_string(),
        };

        Ok(X402Payment {
            x402_version: rsp.x402_version,
            scheme: accept.schema.clone(),
            network: accept.network.clone(),
            payload: X402PaymentPayload {
                signature: signature.to_string(),
                authorization,
            },
        })
    }
}

fn generate_nonce() -> B256 {
    let mut rng = rand::thread_rng();
    let mut nonce_bytes = [0u8; 32];
    rng.fill(&mut nonce_bytes);
    B256::from_slice(&nonce_bytes)
}