<script lang="ts">
  let { name, size = 20, filled = false, flip = false }: { name: string; size?: number; filled?: boolean; flip?: boolean } = $props();

  const P: Record<string, string[]> = {
    home: ['M3 11l9-7 9 7', 'M5 10v10h14V10'],
    tv: ['M3 6h18v12H3z', 'M8 21h8'],
    film: ['M4 4h16v16H4z', 'M8 4v16', 'M16 4v16', 'M4 9h4', 'M4 15h4', 'M16 9h4', 'M16 15h4'],
    layers: ['M12 3l9 5-9 5-9-5z', 'M3 13l9 5 9-5'],
    replay: ['M3 12a9 9 0 1 0 3-6.7', 'M3 4v5h5', 'M10 9l5 3-5 3z'],
    list: ['M8 6h13', 'M8 12h13', 'M8 18h13', 'M3 6h.01', 'M3 12h.01', 'M3 18h.01'],
    sliders: ['M4 7h6', 'M14 7h6', 'M10 7a2 2 0 1 0 4 0a2 2 0 1 0-4 0', 'M4 17h10', 'M18 17h2', 'M14 17a2 2 0 1 0 4 0a2 2 0 1 0-4 0'],
    search: ['M11 4a7 7 0 1 0 0 14a7 7 0 0 0 0-14', 'M16.5 16.5L21 21'],
    star: ['M12 3.5l2.6 5.5 6 .8-4.4 4.2 1.1 6-5.3-2.9-5.3 2.9 1.1-6L3.4 9.8l6-.8z'],
    play: ['M7 4.5v15l12-7.5z'],
    pause: ['M8 5v14', 'M16 5v14'],
    prev: ['M6 5v14', 'M19 5l-9 7 9 7z'],
    next: ['M18 5v14', 'M5 5l9 7-9 7z'],
    expand: ['M4 9V4h5', 'M20 9V4h-5', 'M4 15v5h5', 'M20 15v5h-5'],
    volume: ['M4 9v6h4l5 4V5L8 9z', 'M16.5 8.5a5 5 0 0 1 0 7'],
    cc: ['M3 6h18v12H3z', 'M10 10.5a2 2 0 1 0 0 3', 'M17 10.5a2 2 0 1 0 0 3'],
    back: ['M15 5l-7 7 7 7'],
    chevron: ['M9 5l7 7-7 7'],
    down: ['M6 9l6 6 6-6'],
    check: ['M5 12l5 5 9-10'],
    plus: ['M12 5v14', 'M5 12h14'],
    lock: ['M6 11h12v9H6z', 'M8 11V8a4 4 0 0 1 8 0v3'],
    globe: ['M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0', 'M3 12h18', 'M12 3c3 3 3 15 0 18', 'M12 3c-3 3-3 15 0 18'],
    trash: ['M4 7h16', 'M9 7V4h6v3', 'M6 7l1 13h10l1-13'],
    clock: ['M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0', 'M12 7v5l3 2'],
    user: ['M12 12a4 4 0 1 0 0-8a4 4 0 0 0 0 8', 'M4 20c1-4 4-6 8-6s7 2 8 6'],
    rec: ['M12 7a5 5 0 1 0 0.01 0'],
    grid: ['M4 4h7v7H4z', 'M13 4h7v7h-7z', 'M4 13h7v7H4z', 'M13 13h7v7h-7z'],
    refresh: ['M20 12a8 8 0 1 1-2.3-5.7', 'M20 4v5h-5'],
    eyeoff: ['M3 3l18 18', 'M10.6 6.1A9 9 0 0 1 12 6c5 0 9 6 9 6a15 15 0 0 1-3 3.5', 'M6.5 7.8C4 9.5 3 12 3 12s4 6 9 6c1.2 0 2.3-.3 3.3-.8'],
    x: ['M6 6l12 12', 'M18 6L6 18'],
    guide: ['M3 5h18', 'M3 10h12', 'M3 15h18', 'M3 20h9'],
  };
</script>

<svg
  class="icon"
  class:flip
  viewBox="0 0 24 24"
  width={size}
  height={size}
  fill={filled ? 'currentColor' : 'none'}
  stroke="currentColor"
  stroke-width="1.8"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#each P[name] ?? [] as d}<path {d} />{/each}
</svg>

<style>
  .icon {
    flex: none;
    display: block;
  }
  :global([dir='rtl']) .icon.flip {
    transform: scaleX(-1);
  }
</style>
