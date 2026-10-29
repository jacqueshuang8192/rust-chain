"""Low-frequency, read-only Tron native HTTP capability probe; never accepts keys."""
import json
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

BASE = "https://tron-rpc.publicnode.com"
records = []


def query(method, body):
    time.sleep(1)
    request = urllib.request.Request(
        BASE + "/wallet/" + method,
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "User-Agent": "Tron-capability-check/1.0"},
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            payload = json.load(response)
            records.append({"method": method, "status": response.status})
            return payload
    except (urllib.error.URLError, ValueError) as error:
        records.append({"method": method, "error": str(error)})
        raise


result = {"checked_at": datetime.now(timezone.utc).isoformat(), "endpoint": BASE, "requests": records}
try:
    genesis = query("getblockbynum", {"num": 0})
    result["genesis_mainnet"] = genesis.get("blockID", "").endswith("2b6653dc")
    if not result["genesis_mainnet"]:
        raise ValueError("No recognized Tron mainnet genesis block")
    decimals = query("triggerconstantcontract", {
        "owner_address": "41a614f803b6fd780986a42c78ec9c7f77e6ded13c",
        "contract_address": "41a614f803b6fd780986a42c78ec9c7f77e6ded13c",
        "function_selector": "decimals()", "parameter": "", "visible": False,
    })
    result["usdt_decimals_six"] = decimals.get("result", {}).get("result") is True and int(decimals["constant_result"][0], 16) == 6
    head = query("getnowblock", {})
    header = head["block_header"]["raw_data"]
    result["head_height"] = header["number"]
    result["head_age_seconds"] = round(time.time() - header["timestamp"] / 1000)
    height = header["number"] - 20
    block = query("getblockbynum", {"num": height})
    receipts = query("gettransactioninfobyblocknum", {"num": height})
    ids = {tx["txID"] for tx in block.get("transactions", [])}
    received = {receipt["id"] for receipt in receipts}
    result["sample_height"] = height
    result["transactions"] = len(ids)
    result["receipts"] = len(receipts)
    result["complete_receipts"] = ids == received and len(receipts) == len(ids)
    result["matching_block_identity"] = all(receipt.get("blockNumber") == height and receipt.get("blockTimeStamp") == block["block_header"]["raw_data"]["timestamp"] for receipt in receipts)
    result["passed"] = result["usdt_decimals_six"] and result["complete_receipts"] and result["matching_block_identity"] and 0 <= result["head_age_seconds"] < 120
except Exception as error:
    result["passed"] = False
    result["failure"] = str(error)
Path(__file__).with_name("public-probe.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps(result, indent=2))
