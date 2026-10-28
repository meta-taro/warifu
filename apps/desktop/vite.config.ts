import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  // vitest で Svelte の部品を建てるとき、サーバ用ではなく**ブラウザ用の Svelte**を引く
  // （無いと `mount` が「サーバでは使えない」と落ちる・計画 1-D）
  resolve: process.env.VITEST ? { conditions: ['browser'] } : undefined,
  // Tauri の dev サーバは固定ポートで待つ
  server: { port: 5173, strictPort: true },
  test: {
    // 既定は純ロジック（node）。画面の部品の試験は、そのファイルの先頭で jsdom を指定する
    // （`// @vitest-environment jsdom`・計画 1-D・0.1.41）
    include: ['src/**/*.test.ts'],
    environment: 'node',
  },
});
