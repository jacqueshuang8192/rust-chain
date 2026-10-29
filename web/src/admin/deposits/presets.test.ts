import { expect, it } from 'vitest';
import { assetPresets, hasCredentialPlaceholder } from './presets';

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
