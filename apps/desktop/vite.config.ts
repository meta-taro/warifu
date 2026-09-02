import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  // Tauri の dev サーバは固定ポートで待つ
  server: { port: 5173, strictPort: true },
  test: {
    // 純ロジックだけを見る。DOM を要る所は M5-b 以降
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
