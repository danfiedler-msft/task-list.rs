import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// Same-origin dev: Vite proxies the API + Swagger UI to the axum backend so the SPA can
// call `/api/*` without CORS. The backend binds `0.0.0.0:8080` by default (see AppConfig).
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'http://localhost:8080', changeOrigin: true },
      '/swagger': { target: 'http://localhost:8080', changeOrigin: true },
    },
  },
  build: {
    outDir: 'dist',
  },
  test: {
    environment: 'jsdom',
    setupFiles: './src/setupTests.ts',
    css: false,
  },
});
