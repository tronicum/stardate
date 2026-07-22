import { defineConfig } from 'vite';

export default defineConfig({
  base: './',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
  server: {
    proxy: {
      // During `npm run dev`, proxy tileset requests to a `spex serve` instance
      // running on the default port so the same fetch('/tileset/...') URLs
      // work in both dev and the embedded production build.
      '/tileset': 'http://127.0.0.1:8080',
    },
  },
});
