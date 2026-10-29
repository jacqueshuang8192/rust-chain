"""Bounded, unauthenticated read-only capability probes; no production data."""
import concurrent.futures
import datetime
import json
import time
import urllib.error
import urllib.request
from pathlib import Path

OUT = Path(__file__).with_name('public-rpc-probes.json')
SOL_MAX_VERSION = 0
ENDPOINTS = [
    ('base', 'https://base-rpc.publicnode.com'),
    ('base', 'https://mainnet.base.org'),
    ('eth', 'https://ethereum-rpc.publicnode.com'),
    ('btc', 'https://bitcoin-rpc.publicnode.com'),
    ('solana', 'https://solana-rpc.publicnode.com'),
    ('solana', 'https://api.mainnet-beta.solana.com'),
    ('tron', 'https://api.trongrid.io'),
]


def probe(item):
    network, endpoint = item
    report = {'network': network, 'endpoint': endpoint, 'checks': []}

    def call(method, params, path=''):
        time.sleep(1.1)
        body = params if path else {'jsonrpc': '2.0', 'id': 1, 'method': method, 'params': params}
        request = urllib.request.Request(endpoint+path, json.dumps(body).encode(), {'Content-Type': 'application/json', 'User-Agent': 'rust-chain-capability-check/1.0'})
        start = time.monotonic()
        check = {'method': method}
        report['checks'].append(check)
        try:
            with urllib.request.urlopen(request, timeout=18) as response:
                raw = response.read(32*1024*1024+1)
                if len(raw)>32*1024*1024:
                    raise ValueError('Response exceeds probe bound')
                value=json.loads(raw)
                check.update(http=response.status, bytes=len(raw), elapsed_ms=round((time.monotonic()-start)*1000))
            if isinstance(value, dict) and value.get('error'):
                check['error'] = value['error']
                raise ValueError('RPC rejected method')
            result = value if path else value.get('result')
            check['shape'] = 'array' if isinstance(result, list) else 'object' if isinstance(result, dict) else type(result).__name__
            if isinstance(result,list):
                check['count']=len(result)
            return result
        except urllib.error.HTTPError as error:
            check['http']=error.code
            check['error']=error.read(400).decode(errors='replace')
            raise
        except Exception as error:
            check.setdefault('error', str(error)[:400])
            raise

    try:
        if network in ('base', 'eth'):
            chain=call('eth_chainId', [])
            report['chain_id']=chain
            block=call('eth_getBlockByNumber',['latest', True])
            report['block']={key:block.get(key) for key in ['number','hash','timestamp']}
            report['full_transactions']=all(isinstance(t,dict) for t in block['transactions'])
            report['transaction_count']=len(block['transactions'])
            token='0x833589fcd6edb6e08f4c7c32d4f71b54bda02913' if network=='base' else '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48'
            decimals=call('eth_call',[{'to':token,'data':'0x313ce567'},'latest'])
            report['usdc_decimals']=int(decimals,16)
            logs=call('eth_getLogs',[{'fromBlock':block['number'],'toBlock':block['number'],'address':token,'topics':['0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef']}])
            report['transfer_logs']=len(logs)
            if block['transactions']:
                receipt=call('eth_getTransactionReceipt',[block['transactions'][0]['hash']])
                report['receipt_identity_matches']=receipt['blockHash']==block['hash']
            if network=='eth':
                traces=call('debug_traceBlockByHash',[block['hash'],{'tracer':'callTracer','timeout':'8s'}])
                report['trace_count']=len(traces)
                report['trace_results_complete']=all(isinstance(t.get('result'),dict) for t in traces)
        elif network=='btc':
            info=call('getblockchaininfo',[])
            report['chain']=info['chain']
            report['initialblockdownload']=info['initialblockdownload']
            height=info['blocks']-6
            hash_=call('getblockhash',[height])
            block=call('getblock',[hash_,2])
            report['block_height']=block['height']
            report['transaction_count']=len(block['tx'])
            report['decoded_outputs']=all(isinstance(t,dict) and isinstance(t.get('vout'),list) for t in block['tx'])
        elif network=='solana':
            report['genesis']=call('getGenesisHash',[])
            slot=call('getSlot',[{'commitment':'finalized'}])
            report['slot']=slot
            report['block_time']=call('getBlockTime',[slot])
            block=call('getBlock',[slot,{'commitment':'finalized','encoding':'jsonParsed','transactionDetails':'full','rewards':False,'maxSupportedTransactionVersion':SOL_MAX_VERSION}])
            report['transaction_count']=len(block['transactions'])
            mint=call('getAccountInfo',['EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v',{'encoding':'jsonParsed','commitment':'finalized'}])
            report['usdc_mint_owner']=mint['value']['owner']
            report['usdc_decimals']=mint['value']['data']['parsed']['info']['decimals']
        else:
            genesis=call('getblockbynum',{'num':0},'/wallet/getblockbynum')
            report['genesis_suffix']=genesis.get('blockID','')[-8:]
            block=call('getnowblock',{},'/wallet/getnowblock')
            height=block['block_header']['raw_data']['number']-20
            block=call('getblockbynum',{'num':height},'/wallet/getblockbynum')
            infos=call('gettransactioninfobyblocknum',{'num':height},'/wallet/gettransactioninfobyblocknum')
            report['receipt_ids_match']=set(t['txID'] for t in block.get('transactions',[]))==set(t['id'] for t in infos)
            contract='41a614f803b6fd780986a42c78ec9c7f77e6ded13c'
            decimals=call('triggerconstantcontract',{'owner_address':contract,'contract_address':contract,'function_selector':'decimals()','parameter':'','visible':False},'/wallet/triggerconstantcontract')
            report['usdt_decimals']=int(decimals['constant_result'][0],16)
        report['completed']=True
    except Exception:
        report['completed']=False
    return report

if __name__=='__main__':
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
        reports=list(executor.map(probe,ENDPOINTS))
    result={'checked_at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'single low-volume compatibility sample; no throughput or availability guarantee','providers':reports}
    OUT.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    for r in reports:
        print(json.dumps(r,ensure_ascii=False),flush=True)
