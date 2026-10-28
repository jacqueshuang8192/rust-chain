-- Base uses standard RPC now. Only retire the exact obsolete provider failure;
-- real quotas, key cooldowns, other networks and all scan checkpoints remain.
UPDATE wallet_deposit_scanner_providers
SET cooldown_until=NULL,last_error=NULL,failure_count=0,last_success_at=NULL
WHERE network='base'
  AND last_error='RPC 方法 debug_traceBlockByHash 失败，请检查渠道权限和额度';
