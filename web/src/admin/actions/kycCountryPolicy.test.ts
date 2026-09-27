import { describe, expect, it } from 'vitest';
import { kycCountryPolicyError } from './kycCountryPolicy';

const rules = (countries: string[]) => countries.map((country) => ({ country, document_types: ['passport'] }));
describe('KYC country policy validation', () => {
  it('requires every allowed country to have a rule when rules are enabled', () => {
    expect(kycCountryPolicyError(true, ['China', 'Japan'], rules(['China']))).toContain('Japan');
    expect(kycCountryPolicyError(true, ['CN'], rules(['US']))).toContain('CN');
  });
  it('preserves case-insensitive identities but does not guess aliases', () => {
    expect(kycCountryPolicyError(true, [' China '], rules(['CHINA', 'Japan']))).toBe('');
    expect(kycCountryPolicyError(true, ['CN'], rules(['China']))).not.toBe('');
  });
  it('preserves empty restriction semantics and allows disabling legacy conflicts', () => {
    expect(kycCountryPolicyError(true, [], rules(['CN']))).toBe('');
    expect(kycCountryPolicyError(true, ['CN'], [])).toBe('');
    expect(kycCountryPolicyError(false, ['CN'], rules(['US']))).toBe('');
  });
  it('does not silently drop incomplete rows', () => {
    expect(kycCountryPolicyError(true, [], rules(['']))).toContain('请补全');
    expect(kycCountryPolicyError(true, [], [{ country: 'CN', document_types: [] }])).toContain('请补全');
  });
});
