<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, desc = '', children }: { label: string; desc?: string; children: Snippet } = $props();
</script>

<div class="srow">
  <div class="txt">
    <b>{label}</b>
    {#if desc}<span class="muted">{desc}</span>{/if}
  </div>
  <div class="ctl">{@render children()}</div>
</div>

<style>
  .srow {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 24px;
    min-height: 68px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 200px;
    flex: 1;
  }
  .txt b {
    font-weight: 600;
  }
  .txt .muted {
    font-size: 13px;
    max-width: 60ch;
  }
  .ctl {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
</style>
