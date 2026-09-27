export function kycCountryPolicyError(enabled: boolean, allowed: string[], rules: { country: string; document_types: string[] }[]): string {
  if (rules.some((rule) => !rule.country.trim() || rule.document_types.length === 0)) {
    return '请补全每条国家规则的国家和证件类型，或删除未完成的规则';
  }
  if (!enabled || rules.length === 0) return '';
  const missing = allowed.filter((country) => !rules.some((rule) => rule.country.trim().toLowerCase() === country.trim().toLowerCase()));
  return missing.length ? `以下允许国家缺少证件规则：${missing.join('、')}。国家名称或代码须与规则一致。` : '';
}
