// One self-contained HTML file (fonts and scripts inlined): used to share the
// prototype without a server. The real app uses the normal build.
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { viteSingleFile } from 'vite-plugin-singlefile';

export default defineConfig({
  plugins: [svelte(), viteSingleFile()],
  base: './',
  build: { target: 'es2022', assetsInlineLimit: 100_000_000, outDir: 'dist-single' },
});
