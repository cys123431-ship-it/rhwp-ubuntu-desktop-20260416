import { defineConfig } from 'vite';
import { resolve } from 'path';
import { readFileSync } from 'fs';

const pkg = JSON.parse(readFileSync(resolve(__dirname, 'package.json'), 'utf-8'));

let desktopVersion = '0.1.12'; // fallback
try {
  const desktopPkg = JSON.parse(readFileSync(resolve(__dirname, '..', 'rhwp-desktop', 'package.json'), 'utf-8'));
  desktopVersion = desktopPkg.version;
} catch (e) {
  // ignore
}

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(pkg.version),
    __DESKTOP_VERSION__: JSON.stringify(desktopVersion),
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@wasm': resolve(__dirname, '..', 'pkg'),
    },
  },
  server: {
    host: '0.0.0.0',
    port: 7700,
    allowedHosts: true,
    fs: {
      allow: ['..'],
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('node_modules')) {
            return 'vendor';
          }
          if (id.includes('/src/ui/')) {
            return 'ui-dialogs';
          }
          if (id.includes('/src/engine/')) {
            return 'engine';
          }
        }
      }
    }
  }
});
