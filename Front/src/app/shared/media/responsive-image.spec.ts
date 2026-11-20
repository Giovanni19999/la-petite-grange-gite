import { TestBed } from '@angular/core/testing';

import { PublicImage } from '../../features/public/pages/data-access/pages.models';
import { firstImageBlock } from '../../features/public/pages/ui/blocks/block-renderer';
import { ResponsiveImage } from './responsive-image';

const IMAGE: PublicImage = {
  id: 7,
  width: 1920,
  height: 1280,
  sources: [
    { width: 480, url: '/uploads/abc-480.webp' },
    { width: 960, url: '/uploads/abc-960.webp' },
    { width: 1920, url: '/uploads/abc-1920.webp' },
  ],
  alt: 'La grange au soleil',
  caption: '',
  focal_x: 30,
  focal_y: 70,
};

describe('F003 public images', () => {
  async function render(priority: boolean): Promise<HTMLImageElement> {
    const fixture = TestBed.createComponent(ResponsiveImage);
    fixture.componentRef.setInput('image', IMAGE);
    fixture.componentRef.setInput('sizes', '50vw');
    fixture.componentRef.setInput('priority', priority);
    await fixture.whenStable();
    return fixture.nativeElement.querySelector('img');
  }

  it('F003-S09 uses the WebP widths, the size, the alternative text and the focal point', async () => {
    const img = await render(false);
    expect(img.getAttribute('srcset')).toBe(
      '/uploads/abc-480.webp 480w, /uploads/abc-960.webp 960w, /uploads/abc-1920.webp 1920w',
    );
    expect(img.getAttribute('sizes')).toBe('50vw');
    expect(img.getAttribute('width')).toBe('1920');
    expect(img.getAttribute('height')).toBe('1280');
    expect(img.alt).toBe('La grange au soleil');
    expect(img.style.objectPosition).toBe('30% 70%');
    expect(img.getAttribute('loading')).toBe('lazy');
  });

  it('F003-S09 the first image of the page is loaded at once', async () => {
    const img = await render(true);
    expect(img.getAttribute('loading')).toBe('eager');
    expect(img.getAttribute('fetchpriority')).toBe('high');
  });

  it('F003-S09 finds the first block that shows an image', () => {
    const base = { background: 'background', spacing: 'medium', hide_on_mobile: false } as const;
    expect(
      firstImageBlock([
        {
          ...base,
          id: 'h',
          type: 'hero',
          image: null,
          title: 'T',
          subtitle: '',
          button_label: '',
          button_link: null,
          align: 'center',
          height: 'large',
          darken: 0,
          mode: 'image',
          images: [],
          autoplay: true,
          interval: 6,
        },
        { ...base, id: 't', type: 'text', body: '', width: 'narrow', align: 'left' },
        { ...base, id: 'g', type: 'gallery', images: [IMAGE], columns: 3 },
      ]),
    ).toBe(2);
  });
});
