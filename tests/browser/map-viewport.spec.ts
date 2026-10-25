import { expect, test, type Page } from '@playwright/test';

// Original synthetic fixtures, not source-map artwork. Odd boundary dimensions
// expose stretched edge tiles and wrong downsample/axis conversions.
const levels = Array.from({ length: 6 }, (_, z) => {
  const width = Math.ceil(14310 / 2 ** (5 - z));
  const height = Math.ceil(4632 / 2 ** (5 - z));
  return { z, width, height, columns: Math.ceil(width / 512), rows: Math.ceil(height / 512) };
});
const manifest = { board_width: 14310, board_height: 4632, tile_size: 512, levels };

async function state(page: Page) {
  return page.locator('.research-viewport').evaluate(el => {
    const d = (el as HTMLElement).dataset;
    return { x: +d.centerX!, y: +d.centerY!, zoom: +d.zoom!, level: +d.level!, width: el.clientWidth, height: el.clientHeight };
  });
}

async function open(page: Page) {
  await page.goto('/');
  await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeVisible();
}

test('missing artwork stays explicit and simulation actions do not alter the board', async ({ page }) => {
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'Unavailable' }));
  await open(page);
  await expect(page.locator('.research-status')).toContainText('Artwork unavailable');
  await expect(page.locator('.research-unavailable')).toHaveText('Source artwork is not bundled');
  await expect(page.locator('.research-tile')).toHaveCount(0);
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  const before = await state(page);
  await page.locator('.research-viewport').focus();
  await page.keyboard.press('ArrowRight');
  expect((await state(page)).x - before.x).toBeCloseTo(100, 4);
  await page.getByRole('button', { name: 'End stage — unavailable in review mode', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText('No active simulation');
  await page.keyboard.press('Escape');
  await expect(page.locator('#map-research')).toHaveCount(1);
  expect((await state(page)).x).toBeCloseTo(before.x + 100, 4);
});

test('a small overview covers loading tiles until full detail arrives', async ({ page }) => {
  let release!: () => void;
  const detail = new Promise<void>(resolve => { release = resolve; });
  let overviewRequests = 0;
  let failDetails = false;
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ json: manifest }));
  await page.route(/\/tiles\/map\/\d+\/\d+_\d+\.png$/, async route => {
    if (route.request().url().endsWith('/0/0_0.png')) overviewRequests++;
    else {
      await detail;
      if (failDetails) return route.fulfill({ status: 404, body: 'Missing tile' });
    }
    await route.fulfill({ contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="448" height="145"><rect width="448" height="145" fill="#abbd83"/></svg>' });
  });
  try {
    await page.goto('/', { waitUntil: 'domcontentloaded' });
    await expect(page.locator('.research-board')).toHaveAttribute('data-ready', 'true');
    await expect(page.locator('.research-overview')).toBeVisible();
    await expect(page.locator('.research-tile').first()).not.toHaveAttribute('data-ready', 'true');
    await expect(page.locator('.research-tile').first().locator('span')).toBeHidden();
    await expect(page.locator('.research-tile').first()).toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    release();
    await expect.poll(() => page.locator('.research-tile:not([data-ready=true])').count()).toBe(0);
    failDetails = true;
    await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
    await expect.poll(() => page.locator('.research-tile:not([data-ready=false])').count()).toBe(0);
    await expect(page.locator('.research-tile').first().locator('span')).toBeVisible();
    await expect(page.locator('.research-tile').first()).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
    expect(overviewRequests).toBe(1);
  } finally {
    release();
  }
});

test('tiles follow drag, cursor zoom and a bounded visible ring', async ({ page }, testInfo) => {
  const requests: string[] = [];
  const errors: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ json: manifest }));
  await page.route(/\/tiles\/map\/\d+\/\d+_\d+\.png$/, async route => {
    const url = new URL(route.request().url());
    requests.push(url.pathname);
    const [, z, x, y] = url.pathname.match(/(\d+)\/(\d+)_(\d+)\.png$/)!;
    const level = levels[+z];
    const width = Math.min(512, level.width - +x * 512);
    const height = Math.min(512, level.height - +y * 512);
    await route.fulfill({ contentType: 'image/svg+xml', body: `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"><rect width="100%" height="100%" fill="#dce3d0"/><path d="M0 0H${width}V${height}H0Z" fill="none" stroke="#647653"/><text x="12" y="28" fill="#34422f" font-size="16">SYNTHETIC ${z}:${x}:${y}</text></svg>` });
  });
  await open(page);
  await expect(page.locator('.research-status')).toContainText('Source map loaded');
  await expect(page.locator('.research-unavailable')).toBeHidden();
  await expect(page.locator('.research-tile[data-ready=true]').first()).toBeVisible();
  await page.locator('#map-research').screenshot({ path: testInfo.outputPath('overview.png') });

  // Freeze each interaction before checking requests, so request-time bounds
  // cannot accidentally be compared with a later viewport.
  async function ring() {
    await expect.poll(async () => page.locator('.research-tile:not([data-ready=true])').count()).toBe(0);
    const s = await state(page);
    const l = levels[s.level];
    const sx = l.width / 14310, sy = l.height / 4632;
    const x0 = Math.max(0, Math.floor((s.x - s.width / (2 * s.zoom)) * sx / 512) - 1);
    const x1 = Math.min(l.columns - 1, Math.ceil((s.x + s.width / (2 * s.zoom)) * sx / 512));
    const y0 = Math.max(0, Math.floor((s.y - s.height / (2 * s.zoom)) * sy / 512) - 1);
    const y1 = Math.min(l.rows - 1, Math.ceil((s.y + s.height / (2 * s.zoom)) * sy / 512));
    for (const request of requests.splice(0)) {
      // One tiny full-board overview remains behind the bounded detail ring.
      if (request === '/tiles/map/0/0_0.png') continue;
      const [, z, x, y] = request.match(/(\d+)\/(\d+)_(\d+)\.png$/)!;
      expect(+z).toBe(s.level);
      expect(+x).toBeGreaterThanOrEqual(x0); expect(+x).toBeLessThanOrEqual(x1);
      expect(+y).toBeGreaterThanOrEqual(y0); expect(+y).toBeLessThanOrEqual(y1);
    }
    expect(await page.locator('.research-tile').count()).toBeLessThanOrEqual(100);
  }
  await ring();
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await ring();
  await page.locator('#map-research').screenshot({ path: testInfo.outputPath('gameplay.png') });
  const surface = page.locator('.research-viewport');
  const bounds = (await surface.boundingBox())!;
  const initial = await state(page);
  await page.mouse.move(bounds.x + bounds.width * .75, bounds.y + bounds.height * .5);
  await page.mouse.down();
  // One 500px move lets the network assertion compare one exact viewport.
  await page.mouse.move(bounds.x + bounds.width * .75 - 500, bounds.y + bounds.height * .5);
  await page.mouse.up();
  expect((await state(page)).x - initial.x).toBeCloseTo(500 / initial.zoom, 4);
  await ring();
  const cursor = { x: bounds.width * .37, y: bounds.height * .42 };
  await page.mouse.move(bounds.x + cursor.x, bounds.y + cursor.y);
  for (let i = 0; i < 3; i++) {
    const before = await state(page);
    const anchor = { x: before.x + (cursor.x - before.width / 2) / before.zoom,
      y: before.y + (cursor.y - before.height / 2) / before.zoom };
    await page.mouse.wheel(0, -150);
    await expect.poll(async () => (await state(page)).zoom).toBeGreaterThan(before.zoom);
    const after = await state(page);
    expect(Math.abs(after.x + (cursor.x - after.width / 2) / after.zoom - anchor.x)).toBeLessThan(2);
    expect(Math.abs(after.y + (cursor.y - after.height / 2) / after.zoom - anchor.y)).toBeLessThan(2);
    await ring();
  }
  await surface.focus();
  await page.keyboard.press('Home');
  await ring();
  expect((await state(page)).x).toBeCloseTo(7155, 6);
  await page.getByRole('button', { name: 'Open campaign calendar' }).click();
  await page.keyboard.press('Escape');
  await ring();
  expect(errors).toEqual([]);
});

test('failed tiles retain placeholders and touch pinch preserves its midpoint', async ({ page }, testInfo) => {
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ json: manifest }));
  await page.route(/\/tiles\/map\/\d+\/\d+_\d+\.png$/, route => route.fulfill({ status: 404, body: 'Missing tile' }));
  await open(page);
  await expect(page.locator('.research-tile[data-ready=false]').first()).toBeVisible();
  await expect(page.locator('.research-tile').first()).toContainText('loading or unavailable');
  await expect(page.locator('.research-tile').first().locator('img')).toBeHidden();
  const surface = page.locator('.research-viewport');
  // Fit leaves no horizontal pan room: the camera must not move off board.
  await surface.focus();
  await page.keyboard.press('Home');
  await page.keyboard.press('ArrowLeft');
  expect((await state(page)).x).toBeCloseTo(7155, 6);
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await expect.poll(async () => page.locator('.research-tile:not([data-ready=false])').count()).toBe(0);
  await page.locator('#map-research').screenshot({ path: testInfo.outputPath('missing-tiles.png') });
  const before = await state(page);
  const bounds = (await surface.boundingBox())!;
  const x = bounds.x + bounds.width / 2, y = bounds.y + bounds.height / 2;
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [
    { id: 1, x: x - 100, y }, { id: 2, x: x + 100, y },
  ] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [
    { id: 1, x: x - 200, y }, { id: 2, x: x + 200, y },
  ] });
  await cdp.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  const after = await state(page);
  expect(after.zoom).toBeCloseTo(before.zoom * 2, 3);
  expect(Math.abs(after.x - before.x)).toBeLessThan(2);
  expect(Math.abs(after.y - before.y)).toBeLessThan(2);
  await expect(surface).toHaveAttribute('data-dragging', 'false');
  await page.mouse.move(x + 60, y + 35);
  expect((await state(page)).x).toBe(after.x);
  await page.setViewportSize({ width: 950, height: 720 });
  await expect.poll(async () => (await state(page)).width).toBeLessThan(before.width);
  expect((await state(page)).zoom).toBe(after.zoom);
  await cdp.detach();
});
