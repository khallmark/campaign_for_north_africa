import { readFile, realpath } from 'node:fs/promises';
import { resolve } from 'node:path';
import { defineConfig, loadEnv } from 'vite';

// Explicit local review opt-in. Never copy this source artwork into a build.
export default defineConfig(({ command, mode }) => {
  const directory = loadEnv(mode, process.cwd(), 'CNA_PRIVATE_REVIEW_').CNA_PRIVATE_REVIEW_TILES;
  if (command !== 'serve' || !directory) return {};
  const root = resolve(directory);
  const sourceRoot = resolve('docs/CNAv2.1.0/images');
  return {
    plugins: [{
      name: 'cna-private-review-tiles',
      configureServer(server) {
        server.middlewares.use(async (request, response, next) => {
          const pathname = (request.url ?? '').split('?')[0];
          const token = pathname.startsWith('/__source-tokens/');
          if (!token && !pathname.startsWith('/tiles/map/')) return next();
          response.setHeader('Cache-Control', 'private, no-store');
          response.setHeader('X-Content-Type-Options', 'nosniff');
          // One encoded basename only; never expose arbitrary source paths or SVG scripts.
          let decoded;
          try { decoded = decodeURIComponent(pathname); } catch { decoded = ''; }
          const match = token
            ? /^\/__source-tokens\/([^/\\\x00]+\.(?:png|svg))$/.exec(decoded)
            : /^\/tiles\/map\/(manifest\.json|\d+\/\d+_\d+\.png)$/.exec(pathname);
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
            const file = await realpath(resolve(token ? sourceRoot : root, match[1]));
            if (token && !file.startsWith(`${await realpath(sourceRoot)}/`)) {
              response.statusCode = 404;
              response.end('Not found');
              return;
            }
            const bytes = await readFile(file);
            if (token) response.setHeader('Content-Security-Policy', "default-src 'none'; style-src 'unsafe-inline'; sandbox");
            response.setHeader('Content-Type', match[1].endsWith('.json') ? 'application/json' : match[1].endsWith('.svg') ? 'image/svg+xml' : 'image/png');
            response.end(request.method === 'HEAD' ? undefined : bytes);
          } catch (error) {
            response.statusCode = error.code === 'ENOENT' ? 404 : 500;
            response.end('Review artwork unavailable');
          }
        });
      },
    }],
  };
});
