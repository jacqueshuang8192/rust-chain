//! 持久化渠道/凭据节流与冷却；每个读请求先确认网络租约，进程重启不会清空退避。
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(sqlx::FromRow)]
struct Candidate {
    provider_id: u64,
    key_id: u64,
    endpoint_encrypted: String,
    api_key_encrypted: Option<String>,
    provider_interval: u32,
    key_interval: u32,
}

pub(super) struct Gate {
    stream: String,
    pool: Pool<MySql>,
    network: String,
    revision: u64,
    token: String,
    provider_id: u64,
    key_id: u64,
    provider_interval: u32,
    key_interval: u32,
    pub(super) retryable: AtomicBool,
    channel_failed: AtomicBool,
    pub(super) credential_configured: bool,
}

/// 主备按渠道和 Key 优先级挑选；每次尝试重新做链身份与游标校验，不混用节点区块。
pub(super) async fn scan_available(
    pool: &Pool<MySql>,
    key: Option<&str>,
    config: &ScannerConfig,
    token: &str,
    work: recovery::Work,
) -> Result<(Cursor, u64, bool), recovery::ScanFailure> {
    let encryption_key = key.ok_or_else(|| failure("credential encryption key is missing"))?;
    let candidates=sqlx::query_as::<_,Candidate>("SELECT p.id AS provider_id,k.id AS key_id,p.endpoint_encrypted,k.api_key_encrypted,p.request_interval_ms AS provider_interval,k.request_interval_ms AS key_interval FROM wallet_deposit_scanner_providers p JOIN wallet_deposit_scanner_keys k ON k.provider_id=p.id WHERE p.network=? AND p.enabled=TRUE AND k.enabled=TRUE AND (p.cooldown_until IS NULL OR p.cooldown_until<=CURRENT_TIMESTAMP(6)) AND (k.cooldown_until IS NULL OR k.cooldown_until<=CURRENT_TIMESTAMP(6)) AND NOT EXISTS(SELECT 1 FROM wallet_deposit_scanner_stream_failures f WHERE f.provider_id=p.id AND f.stream=? AND f.cooldown_until>CURRENT_TIMESTAMP(6)) ORDER BY p.priority,p.id,k.priority,k.id")
        .bind(&config.network).bind(&config.stream).fetch_all(pool).await.map_err(AppError::from)?;
    let mut last = failure("所有监听渠道暂不可用或正在冷却，请检查渠道状态后等待自动重试");
    let mut excluded = HashSet::new();
    let mut attempts = 0;
    for candidate in candidates {
        if excluded.contains(&candidate.provider_id) {
            continue;
        }
        if attempts >= 4 {
            break;
        }
        attempts += 1;
        let mut endpoint =
            decrypt_optional_secret(Some(&candidate.endpoint_encrypted), encryption_key)?
                .ok_or_else(|| failure("provider endpoint is missing"))?;
        let mut api_key =
            decrypt_optional_secret(candidate.api_key_encrypted.as_deref(), encryption_key)?;
        if endpoint.contains("{API_KEY}") {
            let secret = api_key
                .take()
                .ok_or_else(|| failure("URL credential template requires an API key"))?;
            let encoded: String = url::form_urlencoded::byte_serialize(secret.as_bytes()).collect();
            endpoint = endpoint.replace("{API_KEY}", &encoded.replace('+', "%20"));
        }
        let node = Node {
            network: config.network.clone(),
            endpoint,
            api_key,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| failure("scanner HTTP client initialization failed"))?,
            gate: Some(Gate {
                stream: config.stream.clone(),
                pool: pool.clone(),
                network: config.network.clone(),
                revision: config.revision,
                token: token.into(),
                provider_id: candidate.provider_id,
                key_id: candidate.key_id,
                provider_interval: candidate.provider_interval,
                key_interval: candidate.key_interval,
                retryable: AtomicBool::new(false),
                channel_failed: AtomicBool::new(false),
                credential_configured: candidate.api_key_encrypted.is_some(),
            }),
        };
        let result =
            tokio::time::timeout(Duration::from_secs(35), scan(pool, config, &node, work)).await;
        let result = match result {
            Ok(result) => result,
            Err(_) => {
                node.gate
                    .as_ref()
                    .unwrap()
                    .failed_scoped("渠道扫描超时，保留进度后切换备用", 30)
                    .await?;
                Err(failure("provider scan timed out"))
            }
        };
        match result {
            Ok(value) => {
                node.gate.as_ref().unwrap().succeeded().await?;
                return Ok(value);
            }
            Err(error) => {
                // 只有传输失败允许自动换源；链身份/区块连续性/账务错误不能被备用节点掩盖。
                if !node
                    .gate
                    .as_ref()
                    .unwrap()
                    .retryable
                    .load(Ordering::Relaxed)
                {
                    return Err(error.into());
                }
                if node
                    .gate
                    .as_ref()
                    .unwrap()
                    .channel_failed
                    .load(Ordering::Relaxed)
                {
                    excluded.insert(candidate.provider_id);
                }
                last = error;
            }
        }
    }
    Err(recovery::ScanFailure {
        error: Box::new(last),
        retryable: true,
    })
}

impl Gate {
    /// 资金提交只读取当前工作租约标识，不暴露节点或凭据。
    pub(super) fn lease_token(&self) -> &str {
        &self.token
    }

    /// 渠道与凭据共用较慢的请求间隔，分流不能叠加服务商额度。
    pub(super) fn interval(&self) -> u32 {
        self.provider_interval.max(self.key_interval)
    }
    async fn lock<'a>(&self, tx: &mut sqlx::Transaction<'a, MySql>) -> AppResult<()> {
        super::super::lock_network(tx, &self.network).await?;
        let source = progress::source(&self.stream);
        let scope = progress::scope(&self.stream);
        let revision = progress::revision(&self.stream);
        let valid: bool=sqlx::query_scalar(&format!("SELECT {revision}=? AND COALESCE(p.lease_token=?,FALSE) AND COALESCE(p.lease_until>CURRENT_TIMESTAMP(6),FALSE) FROM {source} WHERE {scope} FOR UPDATE"))
            .bind(self.revision).bind(&self.token).bind(&self.network).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(failure("scanner lease or configuration changed"));
        }
        Ok(())
    }

    /// 渠道总速率与单 Key 速率同时受控，等待时不持数据库锁；发送前再次校验冷却和租约。
    pub(super) async fn acquire(&self) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        self.lock(&mut tx).await?;
        let (p_next,p_cool): (Option<DateTime<Utc>>,Option<DateTime<Utc>>)=sqlx::query_as("SELECT next_request_at,cooldown_until FROM wallet_deposit_scanner_providers WHERE id=? AND enabled=TRUE FOR UPDATE").bind(self.provider_id).fetch_one(&mut *tx).await?;
        let (k_next,k_cool): (Option<DateTime<Utc>>,Option<DateTime<Utc>>)=sqlx::query_as("SELECT next_request_at,cooldown_until FROM wallet_deposit_scanner_keys WHERE id=? AND enabled=TRUE FOR UPDATE").bind(self.key_id).fetch_one(&mut *tx).await?;
        let now: DateTime<Utc> = sqlx::query_scalar("SELECT CURRENT_TIMESTAMP(6)")
            .fetch_one(&mut *tx)
            .await?;
        if p_cool.is_some_and(|t| t > now) || k_cool.is_some_and(|t| t > now) {
            self.retryable.store(true, Ordering::Relaxed);
            return Err(failure("监听渠道或 API Key 正在冷却"));
        }
        let start = p_next.unwrap_or(now).max(k_next.unwrap_or(now)).max(now);
        sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=? WHERE id=?")
            .bind(start + chrono::Duration::milliseconds(i64::from(self.provider_interval)))
            .bind(self.provider_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE wallet_deposit_scanner_keys SET next_request_at=? WHERE id=?")
            .bind(start + chrono::Duration::milliseconds(i64::from(self.key_interval)))
            .bind(self.key_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        if let Ok(wait) = (start - now).to_std() {
            tokio::time::sleep(wait).await;
        }
        let source = progress::source(&self.stream);
        let scope = progress::scope(&self.stream);
        let revision = progress::revision(&self.stream);
        let eligible:bool=sqlx::query_scalar(&format!("SELECT EXISTS(SELECT 1 FROM {source} JOIN wallet_deposit_scanner_providers pr ON pr.network=p.network JOIN wallet_deposit_scanner_keys k ON k.provider_id=pr.id WHERE {scope} AND {revision}=? AND p.lease_token=? AND p.lease_until>CURRENT_TIMESTAMP(6) AND pr.id=? AND k.id=? AND pr.enabled=TRUE AND k.enabled=TRUE AND (pr.cooldown_until IS NULL OR pr.cooldown_until<=CURRENT_TIMESTAMP(6)) AND (k.cooldown_until IS NULL OR k.cooldown_until<=CURRENT_TIMESTAMP(6)) AND NOT EXISTS(SELECT 1 FROM wallet_deposit_scanner_stream_failures f WHERE f.provider_id=pr.id AND f.stream=? AND f.cooldown_until>CURRENT_TIMESTAMP(6)))"))
            .bind(&self.network).bind(self.revision).bind(&self.token).bind(self.provider_id).bind(self.key_id).bind(&self.stream).fetch_one(&self.pool).await?;
        if !eligible {
            self.retryable.store(true, Ordering::Relaxed);
            return Err(failure("scanner channel changed or entered cooldown"));
        }
        Ok(())
    }

    /// 响应完成后再次持久化间隔，防止数据库或进程延迟耗尽预留时间后形成突发请求。
    pub(super) async fn completed(&self) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        self.lock(&mut tx).await?;
        sqlx::query("UPDATE wallet_deposit_scanner_providers SET next_request_at=GREATEST(COALESCE(next_request_at,CURRENT_TIMESTAMP(6)),DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL ? MICROSECOND)) WHERE id=?")
            .bind(u64::from(self.provider_interval) * 1000).bind(self.provider_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE wallet_deposit_scanner_keys SET next_request_at=GREATEST(COALESCE(next_request_at,CURRENT_TIMESTAMP(6)),DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL ? MICROSECOND)) WHERE id=?")
            .bind(u64::from(self.key_interval) * 1000).bind(self.key_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// 故障冷却与网络租约同一事务；429/401隔离凭据，连接/服务故障隔离整个渠道。
    pub(super) async fn failed(
        &self,
        message: &str,
        provider: bool,
        minimum_seconds: u64,
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        self.lock(&mut tx).await?;
        let (table, id) = if provider {
            ("wallet_deposit_scanner_providers", self.provider_id)
        } else {
            ("wallet_deposit_scanner_keys", self.key_id)
        };
        let count: u32 = sqlx::query_scalar(&format!(
            "SELECT failure_count FROM {table} WHERE id=? FOR UPDATE"
        ))
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let seconds = minimum_seconds
            .max(30u64.saturating_mul(1u64 << count.min(7)))
            .min(7 * 86400);
        sqlx::query(&format!("UPDATE {table} SET cooldown_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL ? SECOND),last_error=?,failure_count=LEAST(failure_count+1,100) WHERE id=?"))
            .bind(seconds).bind(message.chars().take(255).collect::<String>()).bind(id).execute(&mut *tx).await?;
        tx.commit().await?;
        self.retryable.store(true, Ordering::Relaxed);
        self.channel_failed.store(provider, Ordering::Relaxed);
        Ok(())
    }

    /// RPC 能力与整轮超时只冷却当前 Base 分流；真实 HTTP 配额和连接失败仍使用共享冷却。
    pub(super) async fn failed_scoped(&self, message: &str, minimum_seconds: u64) -> AppResult<()> {
        if self.stream.is_empty() {
            return self.failed(message, true, minimum_seconds).await;
        }
        let mut tx = self.pool.begin().await?;
        self.lock(&mut tx).await?;
        sqlx::query("INSERT IGNORE INTO wallet_deposit_scanner_stream_failures(provider_id,stream) VALUES (?,?)")
            .bind(self.provider_id).bind(&self.stream).execute(&mut *tx).await?;
        let count: u32 = sqlx::query_scalar("SELECT failure_count FROM wallet_deposit_scanner_stream_failures WHERE provider_id=? AND stream=? FOR UPDATE")
            .bind(self.provider_id).bind(&self.stream).fetch_one(&mut *tx).await?;
        let seconds = minimum_seconds
            .max(30u64.saturating_mul(1u64 << count.min(7)))
            .min(7 * 86400);
        sqlx::query("UPDATE wallet_deposit_scanner_stream_failures SET cooldown_until=DATE_ADD(CURRENT_TIMESTAMP(6),INTERVAL ? SECOND),last_error=?,failure_count=LEAST(failure_count+1,100) WHERE provider_id=? AND stream=?")
            .bind(seconds).bind(message.chars().take(255).collect::<String>()).bind(self.provider_id).bind(&self.stream).execute(&mut *tx).await?;
        tx.commit().await?;
        self.retryable.store(true, Ordering::Relaxed);
        self.channel_failed.store(true, Ordering::Relaxed);
        Ok(())
    }

    /// 完整扫描尝试成功才清除连续失败计数；普通健康探测不能掩盖后续区块接口的持续失败。
    async fn succeeded(&self) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        self.lock(&mut tx).await?;
        sqlx::query("UPDATE wallet_deposit_scanner_providers SET last_success_at=CURRENT_TIMESTAMP(6),last_error=NULL,cooldown_until=NULL,failure_count=0 WHERE id=? AND (cooldown_until IS NULL OR cooldown_until<=CURRENT_TIMESTAMP(6))").bind(self.provider_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE wallet_deposit_scanner_keys SET last_success_at=CURRENT_TIMESTAMP(6),last_error=NULL,cooldown_until=NULL,failure_count=0 WHERE id=? AND (cooldown_until IS NULL OR cooldown_until<=CURRENT_TIMESTAMP(6))").bind(self.key_id).execute(&mut *tx).await?;
        sqlx::query(
            "DELETE FROM wallet_deposit_scanner_stream_failures WHERE provider_id=? AND stream=?",
        )
        .bind(self.provider_id)
        .bind(&self.stream)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

/// 限流等待接受秒数或 HTTP 日期，始终限制在一分钟到七天之间。
pub(super) fn retry_after(value: Option<&reqwest::header::HeaderValue>) -> u64 {
    let Some(text) = value.and_then(|v| v.to_str().ok()) else {
        return 60;
    };
    text.parse::<u64>()
        .ok()
        .or_else(|| {
            chrono::DateTime::parse_from_rfc2822(text)
                .ok()
                .map(|t| (t.with_timezone(&Utc) - Utc::now()).num_seconds().max(0) as u64)
        })
        .unwrap_or(60)
        .clamp(60, 7 * 86400)
}
