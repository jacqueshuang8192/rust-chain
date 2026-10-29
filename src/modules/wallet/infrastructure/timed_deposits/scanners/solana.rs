//! Solana finalized 区块内真实 System/SPL 指令监听，保留 outer/inner 指令顺序和独立金额。

use super::*;
const SYSTEM: &str = "11111111111111111111111111111111";
const TOKEN: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const MAINNET: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";

/// 仅开放经典 SPL mint；拒绝 Token2022 等具有不同转账/手续费语义的程序，确认单位为 finalized slot 深度。
pub(super) async fn head(node: &Node, assets: &[ChainAsset]) -> AppResult<u64> {
    if node.rpc("getGenesisHash", json!([])).await? != MAINNET {
        return Err(failure("Solana node is not mainnet-beta"));
    }
    for asset in assets.iter().filter(|a| a.asset_kind == "token") {
        let data = node
            .rpc(
                "getAccountInfo",
                json!([asset.token_address,{"encoding":"jsonParsed","commitment":"finalized"}]),
            )
            .await?;
        let value = &data["value"];
        if value["owner"] != TOKEN
            || value["data"]["parsed"]["type"] != "mint"
            || integer(&value["data"]["parsed"]["info"]["decimals"])? != asset.decimals as u64
        {
            return Err(failure(
                "Solana mint program or decimals do not match configuration",
            ));
        }
    }
    let slot = integer(
        &node
            .rpc("getSlot", json!([{"commitment":"finalized"}]))
            .await?,
    )?;
    recent_head(timestamp(integer(
        &node.rpc("getBlockTime", json!([slot])).await?,
    )?)?)?;
    Ok(slot)
}

/// 区块包含完整交易及内部指令；缺少成功交易的执行元数据时停止该块，不把扫描时间当成链时间。
pub(super) async fn block(node: &Node, height: u64, context: &Context) -> AppResult<Block> {
    let data=node.rpc("getBlock",json!([height,{"commitment":"finalized","encoding":"jsonParsed","transactionDetails":"full","rewards":false,"maxSupportedTransactionVersion":1}])).await?;
    let mut block = Block {
        height,
        hash: text(&data["blockhash"])?.into(),
        parent_hash: text(&data["previousBlockhash"])?.into(),
        receipts: vec![],
    };
    let time = timestamp(integer(&data["blockTime"])?)?;
    if context.assets.is_empty() {
        return Ok(block);
    }
    for transaction in array(&data["transactions"])? {
        // V1 仍使用节点解析后的账户与指令；未来未知版本不能静默当成已支持的付款证据。
        if let Some(version) = transaction.get("version")
            && version != "legacy"
            && !matches!(version.as_u64(), Some(0 | 1))
        {
            return Err(failure("unsupported Solana transaction version"));
        }
        let meta = &transaction["meta"];
        if !meta.is_object() || meta.get("err").is_none() {
            return Err(failure(
                "Solana transaction execution metadata is unavailable",
            ));
        }
        if !meta["err"].is_null() {
            continue;
        }
        let tx = &transaction["transaction"];
        let hash = text(&tx["signatures"][0])?;
        let keys = array(&tx["message"]["accountKeys"])?;
        let mut accounts = std::collections::HashMap::new();
        for balance in array(&meta["preTokenBalances"])?
            .iter()
            .chain(array(&meta["postTokenBalances"])?)
        {
            let idx = usize::try_from(integer(&balance["accountIndex"])?)
                .map_err(|_| failure("Solana account index overflow"))?;
            let account = text(
                &keys
                    .get(idx)
                    .ok_or_else(|| failure("Solana balance account missing"))?["pubkey"],
            )?;
            let mint = text(&balance["mint"])?;
            let owner = text(&balance["owner"])?;
            let decimals = integer(&balance["uiTokenAmount"]["decimals"])?;
            let entry = (mint, owner, decimals);
            if accounts
                .insert(account, entry)
                .is_some_and(|old| old != entry)
            {
                return Err(failure(
                    "Solana token account identity changed within transaction",
                ));
            }
        }
        let outer = array(&tx["message"]["instructions"])?;
        let inner = array(&meta["innerInstructions"])?;
        let mut instructions = vec![];
        for (index, instruction) in outer.iter().enumerate() {
            instructions.push(instruction);
            for group in inner
                .iter()
                .filter(|g| g["index"].as_u64() == Some(index as u64))
            {
                instructions.extend(array(&group["instructions"])?);
            }
        }
        for instruction in &instructions {
            let parsed = &instruction["parsed"];
            let info = &parsed["info"];
            if instruction["programId"] == TOKEN
                && matches!(
                    parsed["type"].as_str(),
                    Some("initializeAccount" | "initializeAccount2" | "initializeAccount3")
                )
            {
                let mint = text(&info["mint"])?;
                let owner = text(&info["owner"])?;
                let account = text(&info["account"])?;
                let asset = context
                    .assets
                    .iter()
                    .find(|a| a.asset_kind == "token" && a.token_address == mint);
                if let Some((old_mint, old_owner, old_decimals)) = accounts.get(account) {
                    if *old_mint != mint
                        || *old_owner != owner
                        || asset.is_some_and(|a| a.decimals as u64 != *old_decimals)
                    {
                        return Err(failure("SPL account initialization identity mismatch"));
                    }
                } else {
                    // 非配置 mint 只用于辨认并跳过无关转账；它不参与金额换算。
                    accounts.insert(
                        account,
                        (mint, owner, asset.map_or(u64::MAX, |a| a.decimals as u64)),
                    );
                }
            }
        }
        for (index, instruction) in instructions.iter().enumerate() {
            let Some(program) = instruction["programId"].as_str() else {
                return Err(failure("Solana instruction program ID missing"));
            };
            let parsed = &instruction["parsed"];
            let kind = parsed["type"].as_str().unwrap_or("");
            let info = &parsed["info"];
            let transfer = if program == SYSTEM && matches!(kind, "transfer" | "transferWithSeed") {
                let asset = context.assets.iter().find(|a| a.asset_kind == "native");
                if let Some(asset) = asset {
                    let address = text(&info["destination"])?;
                    if !context.addresses.contains(address) || info["source"] == info["destination"]
                    {
                        continue;
                    }
                    Some((
                        asset,
                        address.to_owned(),
                        raw_amount(&integer(&info["lamports"])?.to_string(), 10, 9)
                            .map_err(|_| failure("invalid Solana transfer amount"))?,
                    ))
                } else {
                    None
                }
            } else if program == TOKEN && matches!(kind, "transfer" | "transferChecked") {
                let destination = text(&info["destination"])?;
                if info["source"] == info["destination"] {
                    continue;
                }
                let Some((mint, owner, decimals)) = accounts.get(destination) else {
                    return Err(failure("SPL transfer destination identity missing"));
                };
                if info["source"]
                    .as_str()
                    .and_then(|source| accounts.get(source))
                    .is_some_and(|(_, source_owner, _)| source_owner == owner)
                {
                    continue;
                }
                let asset = context
                    .assets
                    .iter()
                    .find(|a| a.asset_kind == "token" && a.token_address == *mint);
                if let Some(asset) = asset {
                    if !context.addresses.contains(*owner) {
                        continue;
                    }
                    if *decimals != asset.decimals as u64
                        || (kind == "transferChecked"
                            && (info["mint"] != *mint
                                || integer(&info["tokenAmount"]["decimals"])? != *decimals))
                    {
                        return Err(failure("SPL transfer mint or precision mismatch"));
                    }
                    let raw = if kind == "transferChecked" {
                        text(&info["tokenAmount"]["amount"])?
                    } else {
                        text(&info["amount"])?
                    };
                    Some((
                        asset,
                        (*owner).into(),
                        raw_amount(raw, 10, asset.decimals)
                            .map_err(|_| failure("invalid SPL transfer amount"))?,
                    ))
                } else {
                    None
                }
            } else {
                if matches!(program, SYSTEM | TOKEN) && parsed.is_null() {
                    return Err(failure(
                        "Solana RPC did not parse a supported program instruction",
                    ));
                }
                None
            };
            if let Some((asset, address, amount)) = transfer
                && amount > 0
            {
                block.receipts.push(receipt(
                    context,
                    &block,
                    time,
                    hash,
                    u32::try_from(index)
                        .map_err(|_| failure("Solana instruction index overflow"))?,
                    asset,
                    address,
                    amount,
                )?);
            }
        }
    }
    Ok(block)
}
