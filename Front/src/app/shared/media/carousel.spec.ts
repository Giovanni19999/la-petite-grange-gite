import { ComponentFixture, TestBed } from '@angular/core/testing';

import { PublicImage } from '../../features/public/pages/data-access/pages.models';
import { Carousel } from './carousel';

function photo(id: number, caption = ''): PublicImage {
  return {
    id,
    width: 1600,
    height: 1000,
    sources: [{ width: 960, url: `/uploads/p${id}-960.webp` }],
    alt: `Photo ${id}`,
    caption,
    focal_x: 50,
    focal_y: 50,
  };
}

const PHOTOS = [photo(1, 'La grange'), photo(2), photo(3), photo(4)];

function mockReducedMotion(reduce: boolean): void {
  vi.stubGlobal(
    'matchMedia',
    (query: string) =>
      ({
        matches: reduce && query.includes('reduce'),
        media: query,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
      }) as unknown as MediaQueryList,
  );
}

/** Pixels the photos are dragged by (0 at rest), read from the track transform. */
function offset(track: HTMLElement): number {
  const match = /([+-])\s*(\d+(?:\.\d+)?)px/.exec(track.style.transform);
  return match ? Number(match[2]) * (match[1] === '-' ? -1 : 1) : 0;
}

/** A touch pointer event at (x, y). */
function pointer(target: Element, type: string, x: number, y: number): void {
  target.dispatchEvent(
    new PointerEvent(type, {
      clientX: x,
      clientY: y,
      pointerId: 1,
      pointerType: 'touch',
      button: 0,
      bubbles: true,
    }),
  );
}

/** A horizontal finger move through the given x positions. */
function swipe(target: Element, xs: number[]): void {
  pointer(target, 'pointerdown', xs[0], 100);
  for (const x of xs.slice(1)) {
    pointer(target, 'pointermove', x, 100);
  }
  pointer(target, 'pointerup', xs.at(-1)!, 100);
}

describe('F024 carousel', () => {
  let fixture: ComponentFixture<Carousel>;

  async function render(autoplay: boolean, reduce = false): Promise<HTMLElement> {
    mockReducedMotion(reduce);
    fixture = TestBed.createComponent(Carousel);
    fixture.componentRef.setInput('images', PHOTOS);
    fixture.componentRef.setInput('autoplay', autoplay);
    fixture.componentRef.setInput('interval', 6);
    await fixture.whenStable();
    return fixture.nativeElement as HTMLElement;
  }

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  const current = (root: HTMLElement) =>
    Array.from(root.querySelectorAll('[data-testid="dot"]')).findIndex(
      (dot) => dot.getAttribute('aria-current') === 'true',
    );
  const loadedImages = (root: HTMLElement) =>
    Array.from(root.querySelectorAll('img')).map((img) => img.getAttribute('src'));
  /** Arrow key pressed on the focused carousel. */
  const key = (root: HTMLElement, name: 'ArrowLeft' | 'ArrowRight') =>
    root
      .querySelector('[role="region"]')!
      .dispatchEvent(new KeyboardEvent('keydown', { key: name }));
  const button = (root: HTMLElement, label: string) =>
    root.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`)!;

  async function tick(ms: number): Promise<void> {
    await vi.advanceTimersByTimeAsync(ms);
    await fixture.whenStable();
  }

  it('F024-S02 scrolls by itself, and stops while it has the keyboard focus', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(true);
    expect(current(root)).toBe(0);

    await tick(6000);
    expect(current(root)).toBe(1);

    root.querySelector('[role="region"]')!.dispatchEvent(new FocusEvent('focusin'));
    await fixture.whenStable();
    await tick(12_000);
    expect(current(root)).toBe(1);
  });

  it('F024-S02 has neither on-screen arrows nor a pause button', async () => {
    const root = await render(true);
    expect(root.querySelector('[data-testid="pause"]')).toBeNull();
    expect(button(root, 'Photo suivante')).toBeNull();
    expect(button(root, 'Photo précédente')).toBeNull();
    expect(root.querySelectorAll('[data-testid="dot"]')).toHaveLength(4);
  });

  it('F024-S02 waits a full delay again after the visitor moves', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(true);
    await tick(5000);
    button(root, 'Photo 3 sur 4').click();
    await fixture.whenStable();
    await tick(5000);
    expect(current(root)).toBe(2);
    await tick(1000);
    expect(current(root)).toBe(3);
  });

  it('F024-S02 stops while the pointer is over it', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(true);
    root.querySelector('[role="region"]')!.dispatchEvent(new MouseEvent('mouseenter'));
    await fixture.whenStable();
    await tick(12_000);
    expect(current(root)).toBe(0);
  });

  it('F024-S03 never scrolls by itself when the device asks for reduced motion', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(true, true);
    await tick(20_000);
    expect(current(root)).toBe(0);
    key(root, 'ArrowRight');
    await fixture.whenStable();
    expect(current(root)).toBe(1);
  });

  it('F024-S05 dots, keyboard and swipe move between photos', async () => {
    const root = await render(false);
    key(root, 'ArrowRight');
    await fixture.whenStable();
    expect(current(root)).toBe(1);

    key(root, 'ArrowLeft');
    key(root, 'ArrowLeft');
    await fixture.whenStable();
    expect(current(root)).toBe(3); // wraps around

    button(root, 'Photo 3 sur 4').click();
    await fixture.whenStable();
    expect(current(root)).toBe(2);

    const region = root.querySelector('[role="region"]')!;
    region.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }));
    await fixture.whenStable();
    expect(current(root)).toBe(3);

    swipe(region, [300, 260, 200]);
    await fixture.whenStable();
    expect(current(root)).toBe(0);
  });

  it('F024-S05 on a phone the photos follow the finger, short or vertical moves do nothing', async () => {
    const root = await render(false);
    const region = root.querySelector<HTMLElement>('[role="region"]')!;
    const track = region.firstElementChild as HTMLElement;
    expect(region.className).toContain('touch-pan-y');

    pointer(region, 'pointerdown', 300, 100);
    pointer(region, 'pointermove', 250, 102);
    await fixture.whenStable();
    expect(offset(track)).toBe(-50);
    pointer(region, 'pointercancel', 250, 102);
    await fixture.whenStable();
    expect(offset(track)).toBe(0);
    expect(current(root)).toBe(0);

    swipe(region, [300, 290, 280]);
    await fixture.whenStable();
    expect(current(root)).toBe(0); // too short

    pointer(region, 'pointerdown', 300, 100);
    pointer(region, 'pointermove', 295, 200); // scrolling the page
    await fixture.whenStable();
    expect(offset(track)).toBe(0);

    swipe(region, [100, 160, 220]);
    await fixture.whenStable();
    expect(current(root)).toBe(3); // backwards, wraps around
  });

  it('F024-S05 loops without rewinding: past the last photo it slides on to the first', async () => {
    const root = await render(false);
    const track = root.querySelector('[role="region"]')!.firstElementChild as HTMLElement;
    const copies = () => root.querySelectorAll('[data-testid="slide-copy"]');
    expect(copies()).toHaveLength(2);

    // From the first photo backwards: slides to the copy of the last photo, on the left.
    key(root, 'ArrowLeft');
    await fixture.whenStable();
    expect(current(root)).toBe(3);
    expect(track.style.transform).toContain('(0%');
    expect(copies()[0].querySelector('img')?.getAttribute('src')).toBe('/uploads/p4-960.webp');

    // Once the slide is over, jumps without animation to the real last photo.
    track.dispatchEvent(new Event('transitionend'));
    await fixture.whenStable();
    expect(track.style.transform).toContain('-400%');
    expect(track.classList).not.toContain('transition-transform');

    // From the last photo forwards: slides on to the copy of the first photo, on the right.
    key(root, 'ArrowRight');
    await fixture.whenStable();
    expect(current(root)).toBe(0);
    expect(track.style.transform).toContain('-500%');
    track.dispatchEvent(new Event('transitionend'));
    await fixture.whenStable();
    expect(track.style.transform).toContain('-100%');
  });

  it('F024-S05 a carousel block does not scroll unless asked', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(false);
    await tick(20_000);
    expect(current(root)).toBe(0);
    expect(root.querySelector('[data-testid="pause"]')).toBeNull();
  });

  it('F024-S07 only the photo shown is downloaded until the visitor moves', async () => {
    const root = await render(false);
    expect(loadedImages(root)).toEqual(['/uploads/p1-960.webp']);
    expect(root.querySelector('figcaption')?.textContent).toContain('La grange');

    key(root, 'ArrowRight');
    await fixture.whenStable();
    expect(loadedImages(root)).toEqual(['/uploads/p1-960.webp', '/uploads/p2-960.webp']);
  });

  it('F024-S07 with automatic scrolling, the next photo loads just before it shows', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    const root = await render(true);
    expect(loadedImages(root)).toHaveLength(2);
    expect(loadedImages(root)).not.toContain('/uploads/p3-960.webp');
  });
});
