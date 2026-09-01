import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    host: '0.0.0.0',
    port: 8000,
    proxy: {
      '/health': 'http://localhost:8001',
      '/cities': 'http://localhost:8001',
    },
  },
});
