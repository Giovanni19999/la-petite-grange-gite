import { ComponentRef } from '@angular/core';
import { TestBed } from '@angular/core/testing';

import { Align, ColumnsBlock } from '../../data-access/pages.models';
import { ColumnsBlockView } from './blocks';

function columns(align?: Align): ColumnsBlock {
  return {
    id: 'c',
    type: 'columns',
    background: 'background',
    spacing: 'medium',
    hide_on_mobile: false,
    items: [
      { title: 'Wi-Fi', body: '<p>Fibre</p>' },
      { title: 'Parking', body: '<p>Privé</p>' },
    ],
    align: align as Align,
  };
}

describe('F002 columns block', () => {
  function render(block: ColumnsBlock): HTMLElement[] {
    const fixture = TestBed.createComponent(ColumnsBlockView);
    (fixture.componentRef as ComponentRef<ColumnsBlockView>).setInput('block', block);
    fixture.detectChanges();
    const grid = (fixture.nativeElement as HTMLElement).firstElementChild!;
    return Array.from(grid.children) as HTMLElement[];
  }

  it('F002-S11 aligns the text of every column as chosen', () => {
    for (const [align, css] of [
      ['center', 'text-center'],
      ['right', 'text-right'],
      ['left', 'text-left'],
    ] as const) {
      const items = render(columns(align));
      expect(items.length).toBe(2);
      for (const item of items) {
        expect(item.classList).toContain(css);
        expect(item.classList).toContain('flex');
      }
    }
  });

  it('F002-S11 a block saved before the option stays aligned left', () => {
    for (const item of render(columns(undefined))) {
      expect(item.classList).toContain('text-left');
    }
  });
});
