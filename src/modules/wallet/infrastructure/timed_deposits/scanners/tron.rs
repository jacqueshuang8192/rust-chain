//! Tron FullNode 的已执行 TRC20 日志监听，调用参数本身不能作为到账证据。

use super::*;
use crate::modules::wallet::domain::timed_deposits::{decode_base58, tron_address};

/// 使用创世块 ID 的链标识与合约常量调用验证主网和 decimals，与 java-tron eth_chainId 定义一致。
pub(super) async fn head(node: &Node, assets: &[ChainAsset]) -> AppResult<u64> {
    let genesis = node.post("/wallet/getblockbynum", json!({"num":0})).await?;
    if !text(&genesis["blockID"])?.ends_with("2b6653dc") {
        return Err(failure("Tron node is not mainnet"));
    }
    for asset in assets {
        if asset.asset_kind != "token" {
            return Err(failure(
                "Tron deposits currently require an explicit TRC20 contract",
            ));
        }
        let address =
            decode_base58(&asset.token_address).map_err(|_| failure("invalid TRC20 contract"))?;
        let value=node.post("/wallet/triggerconstantcontract",json!({"owner_address":hex::encode(&address[..21]),"contract_address":hex::encode(&address[..21]),"function_selector":"decimals()","parameter":"","visible":false})).await?;
        if value["result"]["result"] != true {
            return Err(failure("TRC20 decimals call failed"));
        }
        let raw = text(&value["constant_result"][0])?;
        if u32::from_str_radix(raw, 16).ok() != Some(asset.decimals as u32) {
            return Err(failure("TRC20 decimals do not match configuration"));
        }
    }
    let block = node.post("/wallet/getnowblock", json!({})).await?;
    recent_head(timestamp(
        integer(&block["block_header"]["raw_data"]["timestamp"])? / 1000,
    )?)?;
    integer(&block["block_header"]["raw_data"]["number"])
}

/// 从整块交易执行结果读取 Transfer 日志；每个交易必须有执行回执，缺失时整块重试而不推进游标。
pub(super) async fn block(node: &Node, height: u64, context: &Context) -> AppResult<Block> {
    let data = node
        .post("/wallet/getblockbynum", json!({"num":height}))
        .await?;
    let header = &data["block_header"]["raw_data"];
    if integer(&header["number"])? != height {
        return Err(failure("Tron block height mismatch"));
    }
    let mut block = Block {
        height,
        hash: text(&data["blockID"])?.into(),
        parent_hash: text(&header["parentHash"])?.into(),
        receipts: vec![],
    };
    if context.assets.is_empty() {
        return Ok(block);
    }
    let millis = integer(&header["timestamp"])?;
    let time = DateTime::from_timestamp_millis(
        i64::try_from(millis).map_err(|_| failure("invalid Tron timestamp"))?,
    )
    .ok_or_else(|| failure("invalid Tron timestamp"))?;
    let infos = node
        .post(
            "/wallet/gettransactioninfobyblocknum",
            json!({"num":height}),
        )
        .await?;
    let infos = if infos.is_null() {
        return Err(failure("Tron block receipts unavailable"));
    } else {
        array(&infos)?
    };
    let transactions = data["transactions"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let ids: HashSet<_> = transactions
        .iter()
        .map(|t| text(&t["txID"]))
        .collect::<AppResult<_>>()?;
    let received: HashSet<_> = infos
        .iter()
        .map(|t| text(&t["id"]))
        .collect::<AppResult<_>>()?;
    if ids != received || infos.len() != ids.len() {
        return Err(failure("Tron block receipts are incomplete"));
    }
    for info in infos {
        if integer(&info["blockNumber"])? != height || integer(&info["blockTimeStamp"])? != millis {
            return Err(failure("Tron transaction block identity mismatch"));
        }
        if info["receipt"]["result"]
            .as_str()
            .is_some_and(|s| s != "SUCCESS")
            || info["result"].as_str().is_some_and(|s| s == "FAILED")
        {
            continue;
        }
        let Some(logs) = info["log"].as_array() else {
            continue;
        };
        for (index, log) in logs.iter().enumerate() {
            let topics = array(&log["topics"])?;
            if topics.len() != 3 || text(&topics[0])? != TRANSFER_TOPIC {
                continue;
            }
            let contract = tron_address(
                &hex::decode(text(&log["address"])?)
                    .map_err(|_| failure("invalid TRC20 log address"))?,
            )
            .map_err(|_| failure("invalid TRC20 log address"))?;
            let Some(asset) = context
                .assets
                .iter()
                .find(|a| a.asset_kind == "token" && a.token_address == contract)
            else {
                continue;
            };
            let address = super::evm::topic_address("tron", text(&topics[2])?)?;
            if !context.addresses.contains(&address) {
                continue;
            }
            let raw = text(&log["data"])?;
            if raw.len() != 64 {
                return Err(failure("invalid TRC20 transfer value"));
            }
            let amount = raw_amount(raw, 16, asset.decimals)
                .map_err(|_| failure("invalid TRC20 transfer value"))?;
            if amount > 0 {
                block.receipts.push(receipt(
                    context,
                    &block,
                    time,
                    text(&info["id"])?,
                    u32::try_from(index).map_err(|_| failure("Tron log index overflow"))?,
                    asset,
                    address,
                    amount,
                )?);
            }
        }
    }
    Ok(block)
}
