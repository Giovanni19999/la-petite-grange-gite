import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';

import { Channel } from '../data-access/prices.api';
import { PriceRows, PriceTable, pricesFrom, rowsFrom } from './price-table';

const CHANNELS: Channel[] = [
  {
    code: 'direct',
    name: 'Site',
    commission_bp: 0,
    payment_fees_bp: 0,
    vat_on_commission: false,
    effective_bp: 0,
  },
  {
    code: 'airbnb',
    name: 'Airbnb',
    commission_bp: 1550,
    payment_fees_bp: 0,
    vat_on_commission: true,
    effective_bp: 1860,
  },
  {
    code: 'booking',
    name: 'Booking',
    commission_bp: 1500,
    payment_fees_bp: 140,
    vat_on_commission: true,
    effective_bp: 1968,
  },
];

@Component({
  imports: [PriceTable],
  template: `<app-price-table
    [channels]="channels"
    [rows]="rows()"
    (rowsChange)="rows.set($event)"
  />`,
})
class Host {
  readonly channels = CHANNELS;
  readonly rows = signal<PriceRows>(
    rowsFrom(CHANNELS, { direct: { listed: 10_000, net: 10_000 } }),
  );
}

describe('F006 price table', () => {
  function setup() {
    const fixture = TestBed.createComponent(Host);
    fixture.detectChanges();
    const element = fixture.nativeElement as HTMLElement;
    const field = (channel: string, name: 'net' | 'listed') =>
      element.querySelector<HTMLInputElement>(
        `tr[data-channel="${channel}"] [data-field="${name}"]`,
      )!;
    const type = (input: HTMLInputElement, value: string) => {
      input.value = value;
      input.dispatchEvent(new Event('input'));
      fixture.detectChanges();
    };
    return { fixture, field, type };
  }

  it('F006-S04 typing the net price of a channel fills only its listed price', () => {
    const { fixture, field, type } = setup();
    type(field('booking', 'net'), '110');
    expect(field('booking', 'listed').value).toBe('137');
    expect(field('direct', 'listed').value).toBe('100');
    expect(field('airbnb', 'listed').value).toBe('');
    expect(fixture.componentInstance.rows()['direct']).toEqual({ net: '100', listed: '100' });
  });

  it('F006-S04 typing the listed price fills the net price, both stay editable', () => {
    const { fixture, field, type } = setup();
    type(field('booking', 'listed'), '140');
    expect(field('booking', 'net').value).toBe('112');
    type(field('booking', 'net'), '115');
    // The field typed last drives the other one.
    expect(field('booking', 'listed').value).toBe('143');
    type(field('booking', 'listed'), '140');
    const { prices } = pricesFrom(CHANNELS, fixture.componentInstance.rows());
    expect(prices['booking']).toEqual({ listed: 14_000, net: 11_200 });
  });

  it('turns rows into prices and refuses half-filled rows', () => {
    const rows: PriceRows = {
      direct: { net: '100', listed: '100' },
      airbnb: { net: '', listed: '' },
      booking: { net: '120', listed: '' },
    };
    expect(pricesFrom(CHANNELS, rows).error).toBe('Booking : remplissez les deux prix, en euros.');
    rows['booking'] = { net: '150', listed: '140' };
    expect(pricesFrom(CHANNELS, rows).error).toContain('ne peut pas dépasser');
    rows['booking'] = { net: '115', listed: '140' };
    expect(pricesFrom(CHANNELS, rows)).toEqual({
      prices: {
        direct: { listed: 10_000, net: 10_000 },
        airbnb: null,
        booking: { listed: 14_000, net: 11_500 },
      },
      error: null,
    });
  });
});
