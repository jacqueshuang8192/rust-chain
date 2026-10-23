//! 限时充值的精确身份、金额和时间窗规则；不依赖网络或数据库。

use bigdecimal::{
    BigDecimal,
    num_bigint::{BigInt, BigUint},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const RESERVATION_SECONDS: i64 = 1800;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAsset {
    pub id: u64,
    pub asset_symbol: String,
    pub asset_kind: String,
    pub token_address: String,
    pub decimals: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChainReceipt {
    pub network: String,
    pub tx_hash: String,
    pub event_index: u32,
    pub asset_kind: String,
    pub token_address: String,
    pub address: String,
    pub amount: BigDecimal,
    pub block_height: u64,
    pub block_hash: String,
    pub block_time: DateTime<Utc>,
    pub confirmations: u32,
}

/// 用完整的规范文本生成无大小写折叠的数据库查询键；长度分隔防止多段身份拼接碰撞。
pub fn identity_key(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    hex::encode(digest.finalize())
}

/// 链原始整数按明确 decimals 精确缩放；拒绝负值、溢出和不支持的精度，不经过浮点。
pub fn raw_amount(raw: &str, radix: u32, decimals: i32) -> Result<BigDecimal, String> {
    if !(0..=18).contains(&decimals) || raw.len() > 128 || raw.is_empty() {
        return Err("invalid chain amount or decimals".into());
    }
    let integer = BigInt::parse_bytes(raw.as_bytes(), radix).ok_or("invalid chain amount")?;
    if integer < BigInt::from(0) {
        return Err("negative chain amount".into());
    }
    let amount = BigDecimal::new(integer, i64::from(decimals));
    crate::numeric::ensure_amount_storage(&amount, "chain amount")
        .map_err(|_| "chain amount exceeds storage".to_owned())?;
    Ok(amount)
}

/// 地址规范化只折叠不区分大小写的网络；Tron/Solana 使用原始 Base58 大小写，避免地址串户。
pub fn canonical_address(network: &str, value: &str) -> Result<String, String> {
    let value = value.trim();
    let invalid = || "invalid address for deposit network".to_owned();
    match network {
        "eth" | "base" => {
            if value.len() != 42
                || !value.starts_with("0x")
                || !value[2..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(invalid());
            }
            Ok(value.to_ascii_lowercase())
        }
        "tron" => {
            let bytes = decode_base58(value)?;
            if bytes.len() != 25
                || bytes[0] != 0x41
                || Sha256::digest(Sha256::digest(&bytes[..21]))[..4] != bytes[21..]
            {
                return Err(invalid());
            }
            Ok(value.to_owned())
        }
        "solana" => {
            if decode_base58(value)?.len() != 32 {
                return Err(invalid());
            }
            Ok(value.to_owned())
        }
        "btc" => {
            if value.to_ascii_lowercase().starts_with("bc1") {
                validate_segwit(value)?;
                Ok(value.to_ascii_lowercase())
            } else {
                let bytes = decode_base58(value)?;
                if bytes.len() != 25
                    || !matches!(bytes[0], 0 | 5)
                    || Sha256::digest(Sha256::digest(&bytes[..21]))[..4] != bytes[21..]
                {
                    return Err(invalid());
                }
                Ok(value.to_owned())
            }
        }
        _ => Err(invalid()),
    }
}

/// 解析 Base58 字节供地址校验及 Tron 日志收款人转换复用，限制长度以拒绝超大输入。
pub fn decode_base58(value: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    if value.is_empty() || value.len() > 100 {
        return Err("invalid base58 address".into());
    }
    let mut n = BigUint::from(0u32);
    for c in value.bytes() {
        let d = ALPHABET
            .iter()
            .position(|x| *x == c)
            .ok_or("invalid base58 address")?;
        n = n * 58u32 + BigUint::from(d);
    }
    let mut bytes = vec![0; value.bytes().take_while(|b| *b == b'1').count()];
    if n != BigUint::from(0u32) {
        bytes.extend(n.to_bytes_be());
    }
    Ok(bytes)
}

/// 把 Tron 的二十字节地址体编码成带双 SHA256 校验和的主网地址，不接受任意长度日志。
pub fn tron_address(body: &[u8]) -> Result<String, String> {
    if body.len() != 20 {
        return Err("invalid tron address body".into());
    }
    const ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut bytes = vec![0x41];
    bytes.extend(body);
    let checksum = Sha256::digest(Sha256::digest(&bytes));
    bytes.extend(&checksum[..4]);
    let n = BigUint::from_bytes_be(&bytes);
    Ok(n.to_radix_be(58)
        .into_iter()
        .map(|d| ALPHABET[d as usize] as char)
        .collect())
}

fn validate_segwit(value: &str) -> Result<(), String> {
    let fail = || "invalid bitcoin segwit address".to_owned();
    if value.len() > 90
        || (value.bytes().any(|b| b.is_ascii_lowercase())
            && value.bytes().any(|b| b.is_ascii_uppercase()))
    {
        return Err(fail());
    }
    let lower = value.to_ascii_lowercase();
    let data = lower.strip_prefix("bc1").ok_or_else(fail)?;
    const CHARSET: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
    let digits: Vec<u8> = data
        .bytes()
        .map(|b| {
            CHARSET
                .iter()
                .position(|c| *c == b)
                .map(|d| d as u8)
                .ok_or_else(fail)
        })
        .collect::<Result<_, _>>()?;
    if digits.len() < 7 || digits[0] > 16 {
        return Err(fail());
    }
    let mut chk = 1u32;
    for v in [3u8, 3, 0, 2, 3].iter().chain(digits.iter()) {
        let top = chk >> 25;
        chk = ((chk & 0x1ffffff) << 5) ^ u32::from(*v);
        for (i, g) in [0x3b6a57b2, 0x26508e6d, 0x1ea119fa, 0x3d4233dd, 0x2a1462b3]
            .iter()
            .enumerate()
        {
            if (top >> i) & 1 != 0 {
                chk ^= g;
            }
        }
    }
    if chk != if digits[0] == 0 { 1 } else { 0x2bc830a3 } {
        return Err(fail());
    }
    let mut bits = 0u32;
    let mut acc = 0u32;
    let mut length = 0;
    for d in &digits[1..digits.len() - 6] {
        acc = ((acc << 5) | u32::from(*d)) & 0xffff;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            length += 1;
        }
    }
    if bits >= 5
        || (acc & ((1 << bits) - 1)) != 0
        || !(2..=40).contains(&length)
        || (digits[0] == 0 && length != 20 && length != 32)
    {
        return Err(fail());
    }
    Ok(())
}

/// 半开区间以链上时间定位历史订单，迟扫不改变原归属，恰好到期属于过期付款。
pub fn within_window(
    block_time: DateTime<Utc>,
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
) -> bool {
    block_time >= created_at && block_time < expires_at
}

#[cfg(test)]
#[path = "../../../../tests/unit_src/src_modules_wallet_timed_deposit_domain_tests.rs"]
mod tests;
