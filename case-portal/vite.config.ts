/// <reference types="vitest" />
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'path';
import {defineConfig, loadEnv} from 'vite';

export default defineConfig(({mode}) => {
  const env = loadEnv(mode, '.', '');
  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        '@': path.resolve(__dirname, '.'),
      },
    },
    define: {
      'import.meta.env.VITE_AI_ENABLED': JSON.stringify(!!env.GEMINI_API_KEY),
    },
    server: {
      hmr: process.env.DISABLE_HMR !== 'true',
      proxy: {
        '/api': {
          target: env.WOS_SERVER_URL || 'http://localhost:4000',
          changeOrigin: true,
        },
        '/socket.io': {
          target: env.WOS_SERVER_URL || 'http://localhost:4000',
          changeOrigin: true,
          ws: true,
        },
      },
    },
    test: {
      environment: 'jsdom',
      setupFiles: ['./src/setupTests.ts'],
      globals: true,
      exclude: ['e2e/**', 'node_modules/**'],
    },
  };
});
