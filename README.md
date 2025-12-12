# Moss Bounty x402 Client

[![Crates.io](https://img.shields.io/crates/v/moss-bounty-x402-client)](https://crates.io/crates/moss-bounty-x402-client)
[![docs](https://img.shields.io/crates/v/moss-bounty-x402-client?color=yellow&label=docs)](https://docs.rs/moss-bounty-x402-client)

## Installation

```toml
[dependencies]
moss-bounty-x402-client = "0.1.4"
```

## Basic Usage

```rust
use moss_bounty_x402_client::{Client, CreateBountyTaskData};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // User moss authorization token, could be fetched by login
    let auth_token = "<Your-Moss-Authorization-Token>";

    // User wallet private key for bounty payment
    let private = "<Your-Wallet-Private-Key>";
    
    // Build a client
    let client = Client::new(auth_token, private)?;
    
    // Build task data
    let task = CreateBountyTaskData {
        target_twitter_handle: "<target_twitter_handle>".to_string(),
        question: "Hello?".to_string(),
        amount_usdc: "1000000".to_string(),
        valid_hours: 12,
    };

    // Create bounty task
    client.create_bounty_task(task).await?;

    Ok(())
}
```