//! Ethereum/Base 的 ERC20 日志和原生币调用树；区块哈希绑定所有读取，失败调用不产生到账。

use super::*;

/// 校验主网链 ID 和已配置代币 decimals；节点身份或精度错误使整轮停止，不推测资产身份。
pub(super) async fn head(node: &Node, assets: &[ChainAsset]) -> AppResult<u64> {
    let chain = hex_integer(&node.rpc("eth_chainId", json!([])).await?)?;
    if chain != if node.network == "eth" { 1 } else { 8453 } {
        return Err(failure(
            "EVM node chain ID does not match the configured mainnet",
        ));
    }
    for asset in assets.iter().filter(|a| a.asset_kind == "token") {
        let value = node
            .rpc(
                "eth_call",
                json!([{"to":asset.token_address,"data":"0x313ce567"},"latest"]),
            )
            .await?;
        if hex_integer(&value)? != asset.decimals as u64 {
            return Err(failure("EVM token decimals do not match configuration"));
        }
    }
    let latest = node
        .rpc("eth_getBlockByNumber", json!(["latest", false]))
        .await?;
    recent_head(timestamp(hex_integer(&latest["timestamp"])?)?)?;
    hex_integer(&latest["number"])
}

/// 同一块的原生调用树与代币日志使用互不重叠事件索引；节点缺少 trace 能力时禁止开放原生币充值。
pub(super) async fn block(node: &Node, height: u64, context: &Context) -> AppResult<Block> {
    let data = node
        .rpc(
            "eth_getBlockByNumber",
            json!([format!("0x{height:x}"), false]),
        )
        .await?;
    if hex_integer(&data["number"])? != height {
        return Err(failure("EVM block height mismatch"));
    }
    let mut block = Block {
        height,
        hash: text(&data["hash"])?.into(),
        parent_hash: text(&data["parentHash"])?.into(),
        receipts: vec![],
    };
    let time = timestamp(hex_integer(&data["timestamp"])?)?;
    let tokens: Vec<_> = context
        .assets
        .iter()
        .filter(|a| a.asset_kind == "token")
        .collect();
    if !tokens.is_empty() {
        let contracts: Vec<_> = tokens.iter().map(|a| a.token_address.as_str()).collect();
        let logs=node.rpc("eth_getLogs",json!([{"blockHash":block.hash,"address":contracts,"topics":[format!("0x{TRANSFER_TOPIC}")]}])).await?;
        for log in array(&logs)? {
            if log["removed"].as_bool() == Some(true) {
                return Err(failure("removed EVM log returned for confirmed block"));
            }
            if text(&log["blockHash"])? != block.hash || hex_integer(&log["blockNumber"])? != height
            {
                return Err(failure("EVM log block identity mismatch"));
            }
            let contract = canonical_address(&context.network, text(&log["address"])?)
                .map_err(|_| failure("invalid EVM log contract"))?;
            let asset = tokens
                .iter()
                .find(|a| a.token_address == contract)
                .ok_or_else(|| failure("unrequested EVM log contract"))?;
            let topics = array(&log["topics"])?;
            if topics.len() != 3 {
                continue;
            }
            if text(&topics[0])? != format!("0x{TRANSFER_TOPIC}") {
                continue;
            }
            let address = topic_address(&context.network, text(&topics[2])?)?;
            if !context.addresses.contains(&address) {
                continue;
            }
            let index = u32::try_from(hex_integer(&log["logIndex"])?)
                .ok()
                .filter(|i| *i < 0x8000_0000)
                .ok_or_else(|| failure("EVM log index exceeds reserved range"))?;
            let raw = text(&log["data"])?;
            if raw.len() != 66 || !raw.starts_with("0x") {
                return Err(failure("invalid ERC20 transfer amount"));
            }
            let amount = raw_amount(&raw[2..], 16, asset.decimals)
                .map_err(|_| failure("invalid ERC20 transfer amount"))?;
            if amount > 0 {
                block.receipts.push(receipt(
                    context,
                    &block,
                    time,
                    text(&log["transactionHash"])?,
                    index,
                    asset,
                    address,
                    amount,
                )?);
            }
        }
    }
    if let Some(asset) = context.assets.iter().find(|a| a.asset_kind == "native") {
        let traces=node.rpc("debug_traceBlockByHash",json!([block.hash,{"tracer":"callTracer","timeout":"15s","tracerConfig":{"onlyTopCall":false}}])).await?;
        let transactions = array(&data["transactions"])?;
        if array(&traces)?.len() != transactions.len() {
            return Err(failure("EVM block trace is incomplete"));
        }
        for (trace, hash) in array(&traces)?.iter().zip(transactions) {
            let hash = text(hash)?;
            if text(&trace["txHash"])? != hash
                || !trace["result"].is_object()
                || !trace["error"].is_null()
            {
                return Err(failure("EVM transaction trace is incomplete"));
            }
            let mut stack = vec![(&trace["result"], false)];
            let mut index = 0x8000_0000u32;
            while let Some((call, ancestor_failed)) = stack.pop() {
                let current = index;
                index = index
                    .checked_add(1)
                    .ok_or_else(|| failure("EVM trace index overflow"))?;
                let failed = ancestor_failed || !call["error"].is_null();
                if !failed
                    && matches!(
                        text(&call["type"])?,
                        "CALL" | "CREATE" | "CREATE2" | "SELFDESTRUCT"
                    )
                    && !call["value"].is_null()
                {
                    let raw = text(&call["value"])?;
                    let amount = raw_amount(
                        raw.strip_prefix("0x")
                            .ok_or_else(|| failure("invalid EVM native value"))?,
                        16,
                        asset.decimals,
                    )
                    .map_err(|_| failure("invalid EVM native value"))?;
                    if amount > 0 && !call["to"].is_null() {
                        let address = canonical_address(&context.network, text(&call["to"])?)
                            .map_err(|_| failure("invalid EVM trace recipient"))?;
                        if context.addresses.contains(&address)
                            && call["from"].as_str().map(|s| s.to_ascii_lowercase())
                                != Some(address.clone())
                        {
                            block.receipts.push(receipt(
                                context, &block, time, hash, current, asset, address, amount,
                            )?);
                        }
                    }
                }
                if let Some(children) = call["calls"].as_array() {
                    for child in children.iter().rev() {
                        stack.push((child, failed));
                    }
                }
            }
        }
    }
    Ok(block)
}

/// Transfer 收款 topic 必须为左补零的 32 字节地址；不能截断任意输入形成其他收款人。
pub(super) fn topic_address(network: &str, topic: &str) -> AppResult<String> {
    let raw = topic.strip_prefix("0x").unwrap_or(topic);
    if raw.len() != 64
        || !raw.bytes().all(|b| b.is_ascii_hexdigit())
        || !raw[..24].bytes().all(|b| b == b'0')
    {
        return Err(failure("invalid transfer recipient topic"));
    }
    if network == "tron" {
        crate::modules::wallet::domain::timed_deposits::tron_address(
            &hex::decode(&raw[24..]).map_err(|_| failure("invalid Tron recipient"))?,
        )
        .map_err(|_| failure("invalid Tron recipient"))
    } else {
        canonical_address(network, &format!("0x{}", &raw[24..]))
            .map_err(|_| failure("invalid EVM recipient"))
    }
}
