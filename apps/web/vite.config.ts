import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
export default defineConfig({
  plugins: [sveltekit()],
  server: {
    proxy: {
      '/api': `http://127.0.0.1:${process.env.KNOWTOSAY_API_PORT || '4174'}`,
    },
  },
  clearScreen: false,
});
