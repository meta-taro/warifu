import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
export default {
  kit: {
    // Tauri は静的アセットを読む。サーバは無い（DESIGN.md §13）。
    adapter: adapter({ fallback: 'index.html' }),
  },
};
