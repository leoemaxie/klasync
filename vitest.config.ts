import { defineConfig } from 'vitest/config';
import { sveltekit } from '@sveltejs/kit/vite';

// Separate from vite.config.ts (which carries Tauri-specific settings). The
// SvelteKit plugin supplies the `$lib` alias and compiles `.svelte.ts` runes
// modules, so stores can be tested without a browser.
export default defineConfig({
  plugins: [sveltekit()],
  test: {
    include: ['src/**/*.test.ts'],
    environment: 'jsdom',
    restoreMocks: true,
  },
});
