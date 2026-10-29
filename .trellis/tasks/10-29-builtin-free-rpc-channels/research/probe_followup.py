import concurrent.futures, json, time, urllib.request, urllib.error
from pathlib import Path
ROOT=Path(__file__).parent
first=json.loads((ROOT/'public-rpc-probes.json').read_text())['providers']
eth=next(r for r in first if r['network']=='eth')
sol=next(r for r in first if r['network']=='solana')
jobs=[
 ('eth-number','https://ethereum-rpc.publicnode.com','debug_traceBlockByNumber',[eth['block']['number'],{'tracer':'callTracer','timeout':'8s','tracerConfig':{'onlyTopCall':False}}]),
 ('eth-flat','https://ethereum-rpc.publicnode.com','trace_block',[eth['block']['number']]),
 ('solana-v1','https://solana-rpc.publicnode.com','getBlock',[sol['slot'],{'commitment':'finalized','encoding':'jsonParsed','transactionDetails':'full','rewards':False,'maxSupportedTransactionVersion':1}]),
]
def call(job):
 name,url,method,params=job
 time.sleep(1.2)
 result={'name':name,'endpoint':url,'method':method}
 try:
  request=urllib.request.Request(url,json.dumps({'jsonrpc':'2.0','id':1,'method':method,'params':params}).encode(),{'Content-Type':'application/json','User-Agent':'rust-chain-capability-check/1.0'})
  with urllib.request.urlopen(request,timeout=20) as response:
   raw=response.read(32*1024*1024+1)
   assert len(raw)<=32*1024*1024
   data=json.loads(raw)
   result.update(http=response.status,bytes=len(raw))
  if data.get('error'): result['error']=data['error']
  else:
   value=data['result']
   result.update(shape=type(value).__name__,count=len(value))
   if isinstance(value,list): result['first_result']=value[:2]
   if name=='solana-v1':
    result['versions']=dict(__import__('collections').Counter(str(t.get('version')) for t in value['transactions']))
    result['transaction_count']=len(value['transactions'])
    success=[t for t in value['transactions'] if t['meta'] and t['meta']['err'] is None]
    result['successful_metadata_complete']=all(all(k in t['meta'] for k in ['preTokenBalances','postTokenBalances','innerInstructions']) for t in success)
    result['account_keys_parsed']=all(all(isinstance(a,dict) for a in t['transaction']['message']['accountKeys']) for t in success)
    result['v1_sample']=next((t for t in value['transactions'] if t.get('version')==1),None)
 except urllib.error.HTTPError as error:
  result.update(http=error.code,error=error.read(500).decode(errors='replace'))
 except Exception as error: result['error']=str(error)
 return result
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor: results=list(executor.map(call,jobs))
(ROOT/'public-rpc-followup.json').write_text(json.dumps(results,indent=2)+'\n')
for result in results:
 printable={k:v for k,v in result.items() if k not in ['v1_sample','first_result']}
 print(json.dumps(printable),flush=True)
