import { expect, it } from 'vitest';
import { assetPresets, hasCredentialPlaceholder, nodePresets, providerChoiceCount } from './presets';
import { rpcPresetFixture } from '../../../tests/fixtures/rpcPresets';

it('pins issuer mainnet stablecoin identities instead of bridged or testnet variants', () => {
  expect(assetPresets.filter(row => row.kind === 'token').map(row => [row.network, row.symbol, row.token, row.decimals])).toEqual([
    ['eth', 'USDT', '0xdac17f958d2ee523a2206206994597c13d831ec7', 6],
    ['eth', 'USDC', '0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48', 6],
    ['base', 'USDC', '0x833589fcd6edb6e08f4c7c32d4f71b54bda02913', 6],
    ['tron', 'USDT', 'TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t', 6],
    ['solana', 'USDT', 'Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 6],
    ['solana', 'USDC', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 6]
  ]);
});

it.each(['{API_KEY}', 'YOUR_API_KEY', '<API_KEY>', '%7BAPI_KEY%7D'])('blocks unresolved credential marker %s', (marker) => {
  expect(hasCredentialPlaceholder(`https://node.invalid/?key=${marker}`)).toBe(true);
  expect(hasCredentialPlaceholder('https://node.invalid/?key=local-fixture')).toBe(false);
});


it.each(['eth', 'base', 'tron', 'btc', 'solana'])('offers at least five distinct provider choices for %s without counting testing-only public nodes', network => {
  expect(providerChoiceCount(network, rpcPresetFixture)).toBeGreaterThanOrEqual(5);
  const accounts = nodePresets.filter(row => row.network === network && row.account);
  expect(new Set(accounts.map(row => row.account!.provider)).size).toBe(accounts.length);
  for (const row of accounts) {
    expect(row.account!.signup).toMatch(/^https:\/\//);
    expect(row.source).toMatch(/^https:\/\//);
    expect(row.account!.setup).not.toBe('');
    if (row.account!.credential === 'endpoint') expect(row.endpoint).toBe('');
    if (row.account!.credential === 'url-key') expect(row.endpoint).toContain('{API_KEY}');
  }
});

it('counts TronGrid public and account access once and discloses Ethereum paid trace requirements', () => {
  expect(providerChoiceCount('tron', rpcPresetFixture)).toBe(6);
  expect(providerChoiceCount('tron', [])).toBe(5);
  for (const id of ['eth-chainstack', 'eth-alchemy']) expect(nodePresets.find(row => row.id === id)?.account?.plan).toBe('需付费权限');
  expect(nodePresets.find(row => row.id === 'tron-ankr')?.endpoint).toBe('https://rpc.ankr.com/premium-http/tron/{API_KEY}');
  expect(nodePresets.find(row => row.id === 'btc-ankr')?.endpoint).toBe('https://rpc.ankr.com/btc/{API_KEY}');
});

it('counts five documented free Tron providers without counting trials or duplicate public/account access', () => {
  const freeAccounts = nodePresets.filter(row => row.network === 'tron' && row.account?.freeTier);
  expect(freeAccounts.map(row => row.account!.provider).sort()).toEqual(['alchemy', 'ankr', 'chainstack', 'trongrid']);
  expect(providerChoiceCount('tron', rpcPresetFixture, true)).toBe(5);
  expect(providerChoiceCount('tron', [], true)).toBe(4);
  expect(providerChoiceCount('tron', rpcPresetFixture.filter(row => row.id !== 'tron-trongrid-public'), true)).toBe(5);
  expect(providerChoiceCount('tron', rpcPresetFixture.map(row => ({ ...row, recommended: row.id === 'tron-publicnode' ? false : row.recommended })), true)).toBe(4);
  for (const preset of freeAccounts) {
    expect(preset.account!.plan).toBe('免费账号');
    expect(preset.account!.freeTier!.quota).not.toBe('');
    expect(preset.account!.freeTier!.source).toMatch(/^https:\/\//);
  }
  expect(nodePresets.find(row => row.id === 'tron-quicknode')?.account?.freeTier).toBeUndefined();
});
