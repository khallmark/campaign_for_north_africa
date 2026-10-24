import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defineConfig, loadEnv } from 'vite';

// Explicit local review opt-in. Never copy this source artwork into a build.
export default defineConfig(({ command, mode }) => {
  const directory = loadEnv(mode, process.cwd(), 'CNA_PRIVATE_REVIEW_').CNA_PRIVATE_REVIEW_TILES;
  if (command !== 'serve' || !directory) return {};
  const root = resolve(directory);
  return {
    plugins: [{
      name: 'cna-private-review-tiles',
      configureServer(server) {
        server.middlewares.use(async (request, response, next) => {
          const pathname = (request.url ?? '').split('?')[0];
          if (!pathname.startsWith('/tiles/map/')) return next();
          response.setHeader('Cache-Control', 'private, no-store');
          response.setHeader('X-Content-Type-Options', 'nosniff');
          const match = /^\/tiles\/map\/(manifest\.json|\d+\/\d+_\d+\.png)$/.exec(pathname);
          if (!match) {
            response.statusCode = 404;
            response.end('Not found');
            return;
          }
          if (request.method !== 'GET' && request.method !== 'HEAD') {
            response.statusCode = 405;
            response.setHeader('Allow', 'GET, HEAD');
            response.end();
            return;
          }
          try {
            const bytes = await readFile(resolve(root, match[1]));
            response.setHeader('Content-Type', match[1].endsWith('.json') ? 'application/json' : 'image/png');
            response.end(request.method === 'HEAD' ? undefined : bytes);
          } catch (error) {
            response.statusCode = error.code === 'ENOENT' ? 404 : 500;
            response.end('Review tile unavailable');
          }
        });
      },
    }],
  };
});
