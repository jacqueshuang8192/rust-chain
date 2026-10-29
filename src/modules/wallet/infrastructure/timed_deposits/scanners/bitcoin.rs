//! Bitcoin Core 的主网完整区块输出监听；金额保留 JSON 原始十进制，输出序号作为事件身份。

use super::*;

/// 拒绝测试链、初始同步和明显落后的节点；只读取可验证的 Bitcoin Core 主网区块。
pub(super) async fn head(node: &Node) -> AppResult<u64> {
    let info = node.rpc("getblockchaininfo", json!([])).await?;
    if info["chain"] != "main"
        || info["initialblockdownload"].as_bool() != Some(false)
        || integer(&info["headers"])? > integer(&info["blocks"])? + 2
    {
        return Err(failure("Bitcoin node is not synchronized mainnet"));
    }
    integer(&info["blocks"])
}

/// 逐 vout 匹配，不聚合不同输出或分笔款项；同一交易向多个地址付款仍分别保留可去重证据。
pub(super) async fn block(node: &Node, height: u64, context: &Context) -> AppResult<Block> {
    let hash = node.rpc("getblockhash", json!([height])).await?;
    // 游标锚点只需区块身份；实际到账扫描仍读取完整 vout，不以精简响应替代付款证据。
    let verbosity = if context.assets.is_empty() { 1 } else { 2 };
    let data = node.rpc("getblock", json!([hash, verbosity])).await?;
    if data["hash"] != hash
        || integer(&data["height"])? != height
        || data["confirmations"].as_i64().is_none_or(|c| c < 1)
    {
        return Err(failure("Bitcoin block identity mismatch"));
    }
    let mut block = Block {
        height,
        hash: text(&hash)?.into(),
        parent_hash: data["previousblockhash"].as_str().unwrap_or("").into(),
        receipts: vec![],
    };
    let time = timestamp(integer(&data["time"])?)?;
    let Some(asset) = context.assets.iter().find(|a| a.asset_kind == "native") else {
        return Ok(block);
    };
    for tx in array(&data["tx"])? {
        // Coinbase 不是用户转账，不能在 6 确认时当作已可支配充值。
        if array(&tx["vin"])?
            .iter()
            .any(|v| v.get("coinbase").is_some())
        {
            continue;
        }
        for output in array(&tx["vout"])? {
            let Some(address) = output["scriptPubKey"]["address"].as_str() else {
                continue;
            };
            let address = canonical_address("btc", address)
                .map_err(|_| failure("invalid Bitcoin output address"))?;
            if !context.addresses.contains(&address) {
                continue;
            }
            let amount = parse_value(&output["value"])?;
            if amount <= 0 {
                continue;
            }
            let index = u32::try_from(integer(&output["n"])?)
                .map_err(|_| failure("Bitcoin output index overflow"))?;
            block.receipts.push(receipt(
                context,
                &block,
                time,
                text(&tx["txid"])?,
                index,
                asset,
                address,
                amount,
            )?);
        }
    }
    Ok(block)
}

/// arbitrary_precision 保留节点 JSON 数字的全部位数；超过 satoshi 精度的输出直接失败。
pub(super) fn parse_value(value: &Value) -> AppResult<bigdecimal::BigDecimal> {
    if !value.is_number() {
        return Err(failure("Bitcoin output value must be a JSON decimal"));
    }
    let amount = crate::numeric::parse_decimal_input(&value.to_string())
        .map_err(|_| failure("invalid Bitcoin output amount"))?;
    if !crate::modules::wallet::amount_fits_asset_precision(&amount, 8) {
        return Err(failure("Bitcoin output amount exceeds satoshi precision"));
    }
    Ok(amount)
}
