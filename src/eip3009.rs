use alloy::dyn_abi::TypedData;
use alloy::primitives::{Address, B256, U256};
use anyhow::Result;
use serde_json::json;

pub const BASE_CHAIN_ID: u64 = 8453;

pub struct Domain {
    pub name: String,
    pub version: String,
    pub chain_id: u64,
    pub verifying_contract: Address,
}

pub struct Message {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub valid_after: U256,
    pub valid_before: U256,
    pub nonce: B256,
}

pub fn signing_hash(domain: Domain, message: &Message) -> Result<B256> {
    let data = json!({
        "types": {
            "EIP712Domain": [
                {"name": "name", "type": "string"},
                {"name": "version", "type": "string"},
                {"name": "chainId", "type": "uint256"},
                {"name": "verifyingContract", "type": "address"}
            ],
            "TransferWithAuthorization": [
                {"name": "from", "type": "address"},
                {"name": "to", "type": "address"},
                {"name": "value", "type": "uint256"},
                {"name": "validAfter", "type": "uint256"},
                {"name": "validBefore", "type": "uint256"},
                {"name": "nonce", "type": "bytes32"}
            ]
        },
        "domain": {
            "name": domain.name,
            "version": domain.version,
            "chainId": domain.chain_id,
            "verifyingContract": domain.verifying_contract.to_checksum(None),
        },
        "primaryType": "TransferWithAuthorization",
        "message": {
            "from": message.from.to_checksum(None),
            "to": message.to.to_checksum(None),
            "value": message.value.to_string(),
            "validAfter": message.valid_after.to_string(),
            "validBefore": message.valid_before.to_string(),
            "nonce": message.nonce.to_string(),
        }
    });
    let hash = serde_json::from_value::<TypedData>(data)?.eip712_signing_hash()?;
    Ok(hash)
}
