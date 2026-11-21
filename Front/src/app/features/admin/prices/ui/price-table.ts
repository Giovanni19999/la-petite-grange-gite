import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';

import { Channel, ChannelPrice } from '../data-access/prices.api';
import {
  commissionGap,
  formatEuros,
  formatPercent,
  listedFromNet,
  netFromListed,
  parseEuros,
} from '../data-access/price-math';

/** What is typed in a row, as text (so that "110," can be typed). */
export interface PriceRow {
  net: string;
  listed: string;
}

export type PriceRows = Record<string, PriceRow>;

/** Rows of saved prices (empty rows for channels without price). */
export function rowsFrom(
  channels: Channel[],
  prices: Record<string, ChannelPrice | undefined>,
): PriceRows {
  return Object.fromEntries(
    channels.map((channel) => {
      const price = prices[channel.code];
      return [
        channel.code,
        price
          ? { net: formatEuros(price.net), listed: formatEuros(price.listed) }
          : { net: '', listed: '' },
      ];
    }),
  );
}

/**
 * Rows → prices: a row with both fields is a price, an empty row means "no price"; a row with
 * one field or an invalid amount is an error (returned as text).
 */
export function pricesFrom(
  channels: Channel[],
  rows: PriceRows,
): { prices: Record<string, ChannelPrice | null>; error: string | null } {
  const prices: Record<string, ChannelPrice | null> = {};
  for (const channel of channels) {
    const row = rows[channel.code] ?? { net: '', listed: '' };
    if (!row.net.trim() && !row.listed.trim()) {
      prices[channel.code] = null;
      continue;
    }
    const net = parseEuros(row.net);
    const listed = parseEuros(row.listed);
    if (net === null || listed === null) {
      return { prices, error: `${channel.name} : remplissez les deux prix, en euros.` };
    }
    if (listed === 0) {
      return { prices, error: `${channel.name} : le prix affiché doit être supérieur à 0.` };
    }
    if (net > listed) {
      return {
        prices,
        error: `${channel.name} : le prix touché ne peut pas dépasser le prix affiché.`,
      };
    }
    prices[channel.code] = { listed, net };
  }
  return { prices, error: null };
}

/**
 * Price table of the defaults or of a range (F006): one row per channel with "Touché" and
 * "Affiché au client / à saisir". Typing one fills the other of the same row with the
 * channel's rate (ADR-0018); both stay editable; other channels never change.
 */
@Component({
  selector: 'app-price-table',
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="overflow-x-auto">
      <table class="w-full min-w-[22rem] border-collapse text-sm">
        <caption class="sr-only">
          {{
            caption()
          }}
        </caption>
        <thead>
          <tr class="border-b text-left text-muted-foreground">
            <th scope="col" class="py-2 pr-2 font-medium">Canal</th>
            <th scope="col" class="px-2 py-2 font-medium">Touché</th>
            <th scope="col" class="px-2 py-2 font-medium">
              Affiché au client<span class="block text-xs font-normal"
                >(à saisir sur la plateforme)</span
              >
            </th>
          </tr>
        </thead>
        <tbody>
          @for (channel of channels(); track channel.code) {
            @let row = rowOf(channel.code);
            <tr class="border-b align-top" [attr.data-channel]="channel.code">
              <th scope="row" class="py-2 pr-2 text-left font-medium">
                {{ channel.name }}
                <span class="block text-xs font-normal text-muted-foreground">
                  {{
                    channel.effective_bp === 0
                      ? 'sans commission'
                      : 'commission ≈ ' + percent(channel.effective_bp) + ' %'
                  }}
                </span>
              </th>
              <td class="px-2 py-2">
                <label class="flex items-center gap-1">
                  <span class="sr-only">Prix touché {{ channel.name }}</span>
                  <input
                    #net
                    class="h-9 w-24 rounded-md border border-input bg-background px-2 text-right"
                    inputmode="decimal"
                    [placeholder]="placeholder(channel.code, 'net')"
                    [value]="row.net"
                    [attr.aria-invalid]="invalid(row.net) ? 'true' : null"
                    data-field="net"
                    (input)="typeNet(channel, net.value)"
                  />
                  €
                </label>
              </td>
              <td class="px-2 py-2">
                <label class="flex items-center gap-1">
                  <span class="sr-only">Prix affiché {{ channel.name }}</span>
                  <input
                    #listed
                    class="h-9 w-24 rounded-md border border-input bg-background px-2 text-right"
                    inputmode="decimal"
                    [placeholder]="placeholder(channel.code, 'listed')"
                    [value]="row.listed"
                    [attr.aria-invalid]="invalid(row.listed) ? 'true' : null"
                    data-field="listed"
                    (input)="typeListed(channel, listed.value)"
                  />
                  €
                </label>
                @if (gap(channel, row)) {
                  <p class="mt-1 text-xs text-amber-900" role="status">
                    Vérifiez : {{ channel.name }} prend environ
                    {{ percent(channel.effective_bp) }} %.
                  </p>
                }
              </td>
            </tr>
          }
        </tbody>
      </table>
    </div>
  `,
})
export class PriceTable {
  readonly channels = input.required<Channel[]>();
  readonly rows = input.required<PriceRows>();
  /** Warning threshold of the commission, in percentage points. */
  readonly alertPoints = input(5);
  /** Placeholders of the empty rows, e.g. the default prices under a range. */
  readonly fallback = input<Record<string, ChannelPrice>>({});
  readonly caption = input('Prix par canal');
  readonly rowsChange = output<PriceRows>();

  protected readonly percent = formatPercent;

  protected rowOf(code: string): PriceRow {
    return this.rows()[code] ?? { net: '', listed: '' };
  }

  protected placeholder(code: string, field: 'net' | 'listed'): string {
    const fallback = this.fallback()[code];
    return fallback ? formatEuros(fallback[field]) : '';
  }

  protected invalid(text: string): boolean {
    return text.trim() !== '' && parseEuros(text) === null;
  }

  /** Typing the net price fills the listed price of the same channel. */
  protected typeNet(channel: Channel, text: string): void {
    const net = parseEuros(text);
    const listed = net === null ? null : listedFromNet(net, channel.effective_bp);
    this.update(channel.code, {
      net: text,
      listed:
        text.trim() === ''
          ? ''
          : listed === null
            ? this.rowOf(channel.code).listed
            : formatEuros(listed),
    });
  }

  /** Typing the listed price fills the net price of the same channel. */
  protected typeListed(channel: Channel, text: string): void {
    const listed = parseEuros(text);
    this.update(channel.code, {
      listed: text,
      net:
        text.trim() === ''
          ? ''
          : listed === null
            ? this.rowOf(channel.code).net
            : formatEuros(netFromListed(listed, channel.effective_bp)),
    });
  }

  protected gap(channel: Channel, row: PriceRow): boolean {
    const net = parseEuros(row.net);
    const listed = parseEuros(row.listed);
    return (
      channel.effective_bp > 0 &&
      net !== null &&
      listed !== null &&
      commissionGap(listed, net, channel.effective_bp, this.alertPoints())
    );
  }

  private update(code: string, row: PriceRow): void {
    // Always `net` then `listed`, so that rows compare equal as JSON.
    this.rowsChange.emit({ ...this.rows(), [code]: { net: row.net, listed: row.listed } });
  }
}
