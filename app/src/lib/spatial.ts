// Arrow-key navigation by geometry: from the focused control, jump to the nearest
// control in the pressed direction. This is how a TV remote works, and it makes the
// whole app usable without a mouse. Overlays mark themselves with [data-overlay] so
// focus never escapes into the page behind them.

type Dir = 'left' | 'right' | 'up' | 'down';

const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), select:not([disabled]), a[href], [tabindex]:not([tabindex="-1"])';

export function move(dir: Dir): boolean {
  const overlays = document.querySelectorAll('[data-overlay]');
  const scope: ParentNode = overlays.length ? overlays[overlays.length - 1] : document;
  const items = Array.from(scope.querySelectorAll<HTMLElement>(FOCUSABLE)).filter((el) => el.getClientRects().length > 0);
  if (!items.length) return false;

  const cur = document.activeElement as HTMLElement | null;
  if (!cur || !items.includes(cur)) {
    items[0].focus();
    return true;
  }

  const r = cur.getBoundingClientRect();
  const cx = r.left + r.width / 2;
  const cy = r.top + r.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Infinity;

  for (const el of items) {
    if (el === cur) continue;
    const b = el.getBoundingClientRect();
    const dx = b.left + b.width / 2 - cx;
    const dy = b.top + b.height / 2 - cy;
    let primary: number;
    let secondary: number;
    if (dir === 'right') [primary, secondary] = [dx, Math.abs(dy)];
    else if (dir === 'left') [primary, secondary] = [-dx, Math.abs(dy)];
    else if (dir === 'down') [primary, secondary] = [dy, Math.abs(dx)];
    else [primary, secondary] = [-dy, Math.abs(dx)];
    if (primary <= 2) continue;
    const score = primary + secondary * 2.2;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }
  if (!best) return false;
  best.focus({ preventScroll: true });
  best.scrollIntoView({ block: 'nearest', inline: 'nearest' });
  return true;
}
