"""Bounded public reads only. Never load account keys or production configuration."""
import concurrent.futures
import datetime
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

ENDPOINTS = [
    ('tenderly', 'https://mainnet.gateway.tenderly.co'),
    ('0xrpc', 'https://0xrpc.io/eth'),
    ('onfinality', 'https://eth.api.onfinality.io/public'),
    ('alchemy-public', 'https://eth-mainnet.g.alchemy.com/public'),
    ('blast', 'https://eth-mainnet.public.blastapi.io'),
    ('meowrpc', 'https://eth.meowrpc.com'),
]
TOKEN = '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48'
TRANSFER = '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef'


def probe(item):
    name, endpoint = item
    report = {'name': name, 'endpoint': endpoint, 'checks': [], 'complete': False}

    def call(method, params):
        time.sleep(1.5)
        request = urllib.request.Request(endpoint, json.dumps({'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params}).encode(), {'Content-Type': 'application/json', 'User-Agent': ''})
        check = {'method': method}
        report['checks'].append(check)
        start = time.monotonic()
        try:
            with urllib.request.urlopen(request, timeout=15) as response:
                raw = response.read(32 * 1024 * 1024 + 1)
                check.update(http=response.status, elapsed_ms=round((time.monotonic()-start)*1000), bytes=len(raw))
                check['limits'] = {key: value for key, value in response.headers.items() if key.lower().startswith('x-tdly-egress') or key.lower() == 'retry-after'}
                if len(raw) > 32 * 1024 * 1024:
                    raise ValueError('Response exceeds bound')
                value = json.loads(raw)
                if value.get('error'):
                    check['error'] = value['error']
                    raise ValueError('RPC rejected method')
                if value.get('id') != 1:
                    raise ValueError('Wrong RPC response ID')
                return value['result']
        except urllib.error.HTTPError as error:
            check.update(http=error.code, error=error.read(500).decode(errors='replace'))
            raise
        except Exception as error:
            check.setdefault('error', str(error)[:300])
            raise

    try:
        report['chain_id'] = call('eth_chainId', [])
        assert report['chain_id'] == '0x1'
        head = call('eth_getBlockByNumber', ['latest', False])
        block = call('eth_getBlockByNumber', [hex(int(head['number'], 16) - 12), False])
        report['block'] = {key: block[key] for key in ['number', 'hash', 'timestamp']}
        report['head_age_seconds'] = time.time() - int(head['timestamp'], 16)
        traces = call('debug_traceBlockByHash', [block['hash'], {'tracer': 'callTracer', 'timeout': '15s', 'tracerConfig': {'onlyTopCall': False}}])
        report['transaction_count'] = len(block['transactions'])
        report['trace_count'] = len(traces)
        report['trace_identity_complete'] = len(traces) == len(block['transactions']) and all(t.get('txHash') == h and isinstance(t.get('result'), dict) and not t.get('error') for t, h in zip(traces, block['transactions']))
        assert report['trace_identity_complete']
        report['internal_calls'] = sum(bool(t['result'].get('calls')) for t in traces)
        report['usdc_decimals'] = int(call('eth_call', [{'to': TOKEN, 'data': '0x313ce567'}, 'latest']), 16)
        assert report['usdc_decimals'] == 6
        logs = call('eth_getLogs', [{'blockHash': block['hash'], 'address': [TOKEN], 'topics': [TRANSFER]}])
        report['transfer_logs'] = len(logs)
        assert all(log['blockHash'] == block['hash'] and log['blockNumber'] == block['number'] and not log.get('removed', False) for log in logs)
        if block['transactions']:
            receipt = call('eth_getTransactionReceipt', [block['transactions'][0]])
            report['receipt_identity_matches'] = receipt['blockHash'] == block['hash'] and receipt['transactionHash'] == block['transactions'][0] and receipt['blockNumber'] == block['number']
            assert report['receipt_identity_matches']
        report['complete'] = True
    except Exception as error:
        report['failure'] = type(error).__name__
    print(json.dumps(report, ensure_ascii=False), flush=True)
    return report


if __name__ == '__main__':
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
        reports = list(executor.map(probe, ENDPOINTS))
    Path(__file__).with_name('ethereum-probes.json').write_text(json.dumps({'checked_at': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'scope': 'single low-frequency compatibility sample per endpoint; no throughput/SLA claim', 'providers': reports}, ensure_ascii=False, indent=2) + '\n')
