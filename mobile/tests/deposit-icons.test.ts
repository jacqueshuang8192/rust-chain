import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import test from 'node:test'
import { depositAssetIcon, depositNetworkIcon } from '../src/core/depositIcons.ts'

test('five deposit networks use their own vendored logos, including Base rather than ETH', () => {
  for (const network of ['eth', 'base', 'tron', 'btc', 'solana']) {
    const source = depositNetworkIcon(network)
    assert.equal(source, `/deposit-icons/${network === 'btc' ? 'bitcoin' : network}.png`)
    assert.ok(existsSync(new URL(`../public${source}`, import.meta.url)))
  }
  assert.notEqual(depositNetworkIcon('base'), depositAssetIcon('ETH'))
  assert.equal(depositNetworkIcon('unknown'), undefined)
  assert.equal(depositNetworkIcon('constructor'), undefined)
})

test('asset logo fallbacks support nested PWA bases without inventing unknown asset logos', () => {
  for (const symbol of ['ETH', 'TRX', 'SOL', 'BTC', 'USDT', 'USDC']) {
    const source = depositAssetIcon(symbol)
    assert.ok(source && existsSync(new URL(`../public${source}`, import.meta.url)))
  }
  assert.equal(depositAssetIcon('usdt', '/h5/'), '/h5/deposit-icons/usdt.png')
  assert.equal(depositNetworkIcon('tron', '/h5'), '/h5/deposit-icons/tron.png')
  assert.equal(depositAssetIcon('CUSTOM'), undefined)
})
