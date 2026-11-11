import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { readFileSync } from 'node:fs';

// App-owned legal text only. Embedded by Tauri along with the compiled UI.
const notices = ['LICENSE', 'licenses/FlightTrace-MIT.txt', 'licenses/h3sed-MIT.txt', 'licenses/homm3data-MIT.txt', 'licenses/mmarch-MIT.txt'];

export default defineConfig({
  plugins: [react(), {
    name: 'embedded-notices',
    generateBundle() {
      this.emitFile({ type: 'asset', fileName: 'LICENSES.txt', source: notices.map(name => `${name}\n\n${readFileSync(new URL(`../${name}`, import.meta.url), 'utf8')}`).join('\n\n') });
    },
  }],
  clearScreen: false,
  server: { host: '127.0.0.1', port: 1420, strictPort: true },
  build: { target: 'es2022' },
});
