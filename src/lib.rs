mod abi;
mod pb;


use pb::transfers::Transfers;
use pb::approvals::Approvals;
use pb::approvals::ApprovalForAll;
use pb::transfers::TransferBatch;
use pb::transfers::BatchTransfers;
use pb::token::Token;

use substreams::log;
use substreams::scalar::BigInt;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;

use abi::erc1155::events::TransferBatch as TransferBatchEvent;
use abi::erc1155::events::TransferSingle as TransferSingleEvent;
use abi::erc1155::events::ApprovalForAll as ApprovalForAllEvent;

substreams_ethereum::init!();

/// Extracts transfers events from the contract(s)
#[substreams::handlers::map]
fn map_transfers(blk: eth::Block) -> Result<Transfers, substreams::errors::Error> {
    Ok(Transfers {
        transfers: get_transfers(&blk).collect(),
    })
}
#[substreams::handlers::map]
fn map_single_transfers(blk: eth::Block) -> Result<Transfers, substreams::errors::Error> {
    Ok(Transfers {
        transfers: get_single_transfers(&blk).collect(),
    })
}
#[substreams::handlers::map]
fn map_batch_transfers(blk: eth::Block) -> Result<BatchTransfers, substreams::errors::Error> {
    Ok(BatchTransfers {
        transfers: get_batch_transfers(&blk).collect(),
    })
}
#[substreams::handlers::map]
fn map_approvals(blk: eth::Block) -> Result<Approvals, substreams::errors::Error> {
    Ok(Approvals {
        approvals: get_approvals(&blk).collect(),
    })
}


fn get_batch_transfers<'a>(blk: &'a eth::Block) -> impl Iterator<Item = TransferBatch> + 'a {
    blk.receipts().flat_map(|receipt| {
        let hash = &receipt.transaction.hash;

        receipt.receipt.logs.iter().flat_map(|log| {
            if let Some(event) = TransferBatchEvent::match_and_decode(log) {
                return new_erc1155_batch_transfer_simple(hash, log.block_index, event);
            }

            vec![]
        })
    })
}

fn get_single_transfers<'a>(blk: &'a eth::Block) -> impl Iterator<Item = TransferSingle> + 'a {
    blk.receipts().flat_map(|receipt| {
        let hash = &receipt.transaction.hash;

        receipt.receipt.logs.iter().flat_map(|log| {
            if let Some(event) = TransferSingleEvent::match_and_decode(log) {
                return vec![new_erc1155_single_transfer(hash, log.block_index, event)];
            }

            vec![]
        })
    })
}


fn get_transfers<'a>(blk: &'a eth::Block) -> impl Iterator<Item = Transfer> + 'a {
    blk.receipts().flat_map(|receipt| {
        let hash = &receipt.transaction.hash;

        receipt.receipt.logs.iter().flat_map(|log| {
            if let Some(event) = TransferSingleEvent::match_and_decode(log) {
                return vec![new_erc1155_single_transfer(hash, log.block_index, event)];
            }

            if let Some(event) = TransferBatchEvent::match_and_decode(log) {
                return new_erc1155_batch_transfer(hash, log.block_index, event);
            }

            vec![]
        })
    })
}

fn get_approvals<'a>(blk: &'a eth::Block) -> impl Iterator<Item = ApprovalForAll> + 'a {
    blk.receipts().flat_map(|receipt| {
        let hash = &receipt.transaction.hash;

        receipt.receipt.logs.iter().flat_map(|log| {
            if let Some(event) = ApprovalForAllEvent::match_and_decode(log) {
                return vec![new_erc1155_approval(hash, log.block_index, event)];
            }
            vec![]
        })
    })
}


fn new_erc1155_approval(
    hash: &[u8],
    log_index: u32,
    event: ApprovalForAllEvent,
) -> ApprovalForAll {
    ApprovalForAll {
        trx_hash: Hex(hash).to_string(),
        log_index: log_index as u64,
        account: Hex(event.account).to_string(),
        operator: Hex(event.operator).to_string(),
        approved: event.approved
    }
}


fn new_erc1155_single_transfer(
    hash: &[u8],
    log_index: u32,
    event: TransferSingleEvent,
) -> TransferSingle {
    new_erc1155_transfer(
        hash,
        log_index,
        &event.from,
        &event.to,
        &event.id,
        &event.value,
        &event.operator,
    )
}

fn new_erc1155_batch_transfer_simple(
    hash: &[u8],
    log_index: u32,
    from: &[u8],
    to: &[u8],
    token_ids: &[BigInt],
    quantitys: &[BigInt],
    operator: &[u8],
) -> TransferBatch {
    TransferBatch {
        from: Hex(from).to_string(),
        to: Hex(to).to_string(),
        quantitys:  quantitys
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                            id.to_string();
                        })
                        .collect(),
        trx_hash: Hex(hash).to_string(),
        log_index: log_index as u64,
        operator: Hex(operator).to_string(),
        token_ids: token_ids
                        .iter()
                        .enumerate()
                        .map(|(i, id)| {
                           id.to_string();
                        })
                        .collect(),
    }
}


fn new_erc1155_batch_transfer(
    hash: &[u8],
    log_index: u32,
    event: TransferBatchEvent,
) -> Vec<Transfer> {
    if event.ids.len() != event.values.len() {
        log::info!("There is a different count for ids ({}) and values ({}) in transaction {} for log at block index {}, ERC1155 spec says length should match, ignoring the log completely for now",
            event.ids.len(),
            event.values.len(),
            Hex(&hash).to_string(),
            log_index,
        );

        return vec![];
    }

    event
        .ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let value = event.values.get(i).unwrap();

            new_erc1155_transfer(
                hash,
                log_index,
                &event.from,
                &event.to,
                id,
                value,
                &event.operator,
            )
        })
        .collect()
}



fn new_erc1155_transfer(
    hash: &[u8],
    log_index: u32,
    from: &[u8],
    to: &[u8],
    token_id: &BigInt,
    quantity: &BigInt,
    operator: &[u8],
) -> Transfer {
    Transfer {
        from: Hex(from).to_string(),
        to: Hex(to).to_string(),
        quantity: quantity.to_string(),
        trx_hash: Hex(hash).to_string(),
        log_index: log_index as u64,
        operator: Hex(operator).to_string(),
        token_id: token_id.to_string(),
    }
}

//Stores
