import vercelAdapter from '@sveltejs/adapter-vercel';
import staticAdapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const isVercel = Boolean(process.env.VERCEL);
const isTauri = Object.keys(process.env).some((key) => key.startsWith('TAURI_'));
const isStatic = Boolean(
  process.env.TAURI_ENV ||
  process.env.BUILD_TARGET === 'static' ||
  isTauri ||
  !isVercel
);

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: isStatic
      ? staticAdapter({
          pages: 'build',
          assets: 'build',
          fallback: 'index.html',
          precompress: false,
          strict: false,
        })
      : vercelAdapter({
          runtime: 'nodejs20.x',
          images: {
            sizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
            domains: [],
            minimumCacheTTL: 60,
            formats: ['image/avif', 'image/webp'],
          },
        }),
  },
};

export default config;
