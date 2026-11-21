/**
 * Calculations of the price screen (F006, ADR-0018), the same formulas as the API domain
 * (`domain::pricing`): amounts in cents, rates in basis points (1968 = 19.68 %). They only fill
 * the fields; what is saved is what the fields show.
 */

const FULL = 10_000;

/** Rounded to the euro, like the API. */
function roundToEuro(cents: number): number {
  return Math.floor((cents + 50) / 100) * 100;
}

/** Listed price filled from a net price: `net / (1 − t)`; `null` if the rate is 100 %. */
export function listedFromNet(net: number, effectiveBp: number): number | null {
  const keep = FULL - effectiveBp;
  return keep > 0 ? roundToEuro(Math.floor((net * FULL) / keep)) : null;
}

/** Net price filled from a listed price: `listed × (1 − t)`. */
export function netFromListed(listed: number, effectiveBp: number): number {
  return roundToEuro(Math.floor((listed * (FULL - effectiveBp)) / FULL));
}

/** Whether the implicit commission differs from the channel's rate by more than `points`. */
export function commissionGap(
  listed: number,
  net: number,
  effectiveBp: number,
  points: number,
): boolean {
  if (listed <= 0) {
    return false;
  }
  const implicit = Math.floor(((listed - Math.min(net, listed)) * FULL) / listed);
  return Math.abs(implicit - effectiveBp) > points * 100;
}

/** "110" or "110,50" (or "110.5") in euros → cents; `null` if empty or not a price. */
export function parseEuros(text: string): number | null {
  const cleaned = text.replace(/\s|€/g, '').replace(',', '.');
  if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) {
    return null;
  }
  return Math.round(Number(cleaned) * 100);
}

/** Cents → "110" or "110,50". */
export function formatEuros(cents: number): string {
  return cents % 100 === 0
    ? String(cents / 100)
    : `${Math.floor(cents / 100)},${String(cents % 100).padStart(2, '0')}`;
}

/** Cents → "110 €" or "110,50 €". */
export function euros(cents: number): string {
  return `${formatEuros(cents)} €`;
}

/** Basis points → "15,5". */
export function formatPercent(bp: number): string {
  return (bp / 100).toLocaleString('fr-FR', { maximumFractionDigits: 2 });
}

/** "15,5" → 1550; `null` if not a percentage between 0 and 100. */
export function parsePercent(text: string): number | null {
  const value = Number(text.replace(',', '.').trim());
  return text.trim() !== '' && Number.isFinite(value) && value >= 0 && value <= 100
    ? Math.round(value * 100)
    : null;
}
