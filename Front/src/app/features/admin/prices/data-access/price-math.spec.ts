import {
  commissionGap,
  formatEuros,
  formatPercent,
  listedFromNet,
  netFromListed,
  parseEuros,
  parsePercent,
} from './price-math';

describe('F006 price math (same results as the API)', () => {
  it('F006-S04 fills the listed price from the net price, rounded to the euro', () => {
    expect(listedFromNet(11_000, 1968)).toBe(13_700);
    expect(listedFromNet(10_000, 0)).toBe(10_000);
    expect(listedFromNet(10_000, 10_000)).toBeNull();
  });

  it('F006-S04 fills the net price from the listed price', () => {
    expect(netFromListed(14_000, 1968)).toBe(11_200);
    expect(netFromListed(9_900, 0)).toBe(9_900);
  });

  it('warns when the implicit commission is far from the rate', () => {
    expect(commissionGap(14_000, 11_500, 1968, 5)).toBe(false);
    expect(commissionGap(14_000, 14_000, 1968, 5)).toBe(true);
  });

  it('parses and formats euros and percentages', () => {
    expect(parseEuros('110')).toBe(11_000);
    expect(parseEuros('110,50 €')).toBe(11_050);
    expect(parseEuros('abc')).toBeNull();
    expect(parseEuros('')).toBeNull();
    expect(formatEuros(11_050)).toBe('110,50');
    expect(formatEuros(11_000)).toBe('110');
    expect(parsePercent('15,5')).toBe(1550);
    expect(parsePercent('120')).toBeNull();
    expect(formatPercent(1968)).toBe('19,68');
  });
});
