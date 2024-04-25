use ethers_core::types::{U256, U64};
use ic_cdk::println;

use crate::{
    evm_rpc::{
        MultiSendRawTransactionResult, RpcServices, SendRawTransactionResult,
        SendRawTransactionStatus, EVM_RPC,
    },
    evm_signer::{self, SignRequest},
    fees::{self, FeeEstimates},
    state::{mutate_state, read_state},
};

pub async fn transfer_eth_from_canister(value: u128, to: String) {
    let FeeEstimates {
        max_fee_per_gas,
        max_priority_fee_per_gas,
    } = fees::estimate_transaction_fees(9).await;
    let nonce = read_state(|s| s.nonce);
    let rpc_providers = read_state(|s| s.rpc_services.clone());

    let req = SignRequest {
        chain_id: Some(rpc_providers.chain_id()),
        to: Some(to),
        from: None,
        gas: Some(U256::from(21000)),
        max_fee_per_gas: Some(max_fee_per_gas),
        max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
        data: None,
        value: Some(U256::from(value)),
        nonce: Some(U256::from(nonce)),
    };

    let tx = evm_signer::sign_transaction(req).await;

    let status = send_raw_transaction(tx.clone()).await;

    println!("Transaction sent: {:?}", tx);

    match status {
        SendRawTransactionStatus::Ok(transaction_hash) => {
            println!("Success {transaction_hash:?}");
            mutate_state(|s| {
                s.nonce += 1;
            });
        }
        SendRawTransactionStatus::NonceTooLow => {
            println!("Nonce too low");
        }
        SendRawTransactionStatus::NonceTooHigh => {
            println!("Nonce too high");
        }
        SendRawTransactionStatus::InsufficientFunds => {
            println!("Insufficient funds");
        }
    }
}

pub async fn send_raw_transaction(tx: String) -> SendRawTransactionStatus {
    let rpc_providers = read_state(|s| s.rpc_services.clone());
    let cycles = 10_000_000_000;

    match EVM_RPC
        .eth_send_raw_transaction(rpc_providers, None, tx, cycles)
        .await
    {
        Ok((res,)) => match res {
            MultiSendRawTransactionResult::Consistent(status) => match status {
                SendRawTransactionResult::Ok(status) => status,
                SendRawTransactionResult::Err(e) => {
                    ic_cdk::trap(format!("Error: {:?}", e).as_str());
                }
            },
            MultiSendRawTransactionResult::Inconsistent(_) => {
                ic_cdk::trap("Status is inconsistent");
            }
        },
        Err(e) => ic_cdk::trap(format!("Error: {:?}", e).as_str()),
    }
}

impl RpcServices {
    pub fn chain_id(&self) -> U64 {
        match self {
            RpcServices::EthSepolia(_) => U64::from(11155111),
            RpcServices::Custom {
                chainId,
                services: _,
            } => U64::from(*chainId),
            RpcServices::EthMainnet(_) => U64::from(1),
        }
    }
}
