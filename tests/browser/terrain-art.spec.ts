import { expect, test } from '@playwright/test';

test('map legend explains active symbols and scrolls without moving the map', async ({ page }) => {
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'No private scan' }));
  await page.goto('/');
  const summary = page.locator('.geography-legend > summary');
  const legend = page.getByRole('region', { name: 'Map legend', exact: true });
  const camera = () => page.locator('.research-viewport').evaluate((el: HTMLElement) =>
    [el.dataset.centerX, el.dataset.centerY, el.dataset.zoom]);
  await expect(summary).toBeVisible();
  await expect(legend).toBeHidden();
  const initialCamera = await camera();
  await summary.focus();
  await page.keyboard.press('Enter');
  await expect(legend).toBeVisible();
  await expect(legend.getByRole('heading')).toHaveText([
    'Terrain tiles', 'Routes & construction', 'Water & hexsides', 'Places & facilities', 'Unclassified evidence', 'Hex grid',
  ]);
  await expect(legend).toContainText('Not Clear');
  await expect(legend).toContainText('excluded from network links');
  await expect(legend).toContainText('not surveyed boundaries');
  // All samples are real bundled assets, including below-the-fold lazy images.
  expect(await legend.locator('img').evaluateAll(async images => {
    return Promise.all(images.map(async (node: HTMLImageElement) => {
      node.loading = 'eager';
      await node.decode();
      return node.naturalWidth > 0;
    }));
  })).not.toContain(false);
  await expect(legend.locator('.legend-route.railroad').first()).toHaveCSS('stroke', 'rgb(41, 61, 58)');
  await expect(legend.locator('.legend-construction.unfinished-railroad')).toHaveCSS('stroke', 'rgb(113, 68, 160)');
  await legend.hover();
  await page.mouse.wheel(0, 450);
  await expect.poll(() => legend.evaluate(el => el.scrollTop)).toBeGreaterThan(0);
  expect(await camera()).toEqual(initialCamera);
  await legend.focus();
  await page.keyboard.press('End');
  await expect(legend.getByText(/Only enabled layers/)).toBeInViewport();
  await page.getByRole('button', { name: 'Original terrain', exact: true }).click();
  await expect(legend.getByRole('heading', { name: 'Terrain tiles', exact: true })).toBeHidden();
  await expect(legend.getByRole('heading', { name: 'Terrain evidence tints', exact: true })).toBeVisible();
  await expect(legend.getByRole('heading', { name: 'Locations', exact: true })).toBeVisible();
  await expect(legend.getByRole('heading', { name: 'Places & facilities', exact: true })).toBeHidden();
  await page.getByRole('button', { name: 'Geography', exact: true }).click();
  await expect(legend.getByRole('heading')).toHaveText(['Hex grid']);
  await page.getByRole('button', { name: 'Hex grid', exact: true }).click();
  await expect(legend.locator('.legend-empty')).toBeVisible();
  await page.getByRole('button', { name: 'Inspect mode', exact: true }).click();
  await expect(legend.getByRole('heading')).toHaveText(['Inspect mode', 'Inspected hex']);
  await expect(legend.locator('.legend-empty')).toBeHidden();
  await page.getByRole('button', { name: 'Inspect mode', exact: true }).click();
  await page.getByRole('button', { name: 'Original terrain', exact: true }).click();
  await expect(legend.getByRole('heading')).toHaveText(['Terrain tiles']);
  expect(await camera()).toEqual(initialCamera);
  await summary.focus();
  await page.keyboard.press('Space');
  await expect(legend).toBeHidden();
  await summary.click();
  await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill('C4217');
  await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
  await expect(page.locator('.geography-inspector')).toContainText('C4217');
  await expect(legend).toBeHidden();

  await page.setViewportSize({ width: 390, height: 844 });
  await summary.click();
  await expect(legend).toBeVisible();
  await expect(page.locator('.geography-inspector')).toBeHidden();
  const tools = page.locator('.geography-tools');
  const box = (await tools.boundingBox())!;
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(390);
  expect(box.y).toBeGreaterThanOrEqual(0);
  expect(box.y + box.height).toBeLessThan(844);
  for (const panel of await page.locator('.floating-panel').all()) {
    const panelBox = (await panel.boundingBox())!;
    expect(box.y).toBeGreaterThan(panelBox.y + panelBox.height);
  }
  expect(await tools.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await legend.focus();
  await page.keyboard.press('End');
  await expect(legend.getByText(/Only enabled layers/)).toBeInViewport();
  await summary.focus();
  await page.keyboard.press('Enter');
  await expect(legend).toBeHidden();
});

test('bundled terrain works without private scans and survives map controls', async ({ page }) => {
  const failedArt: string[] = [];
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('response', response => {
    if (response.url().includes('/art/terrain-v1/') && !response.ok()) failedArt.push(response.url());
  });
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'No private scan' }));
  await page.goto('/');
  const art = page.getByRole('button', { name: 'Original terrain', exact: true });
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('[data-map-control=source]')).toBeDisabled();
  await expect(page.locator('[data-map-control=source]')).toBeHidden();
  await expect(page.locator('.research-status')).toContainText('Original terrain art');
  await expect(page.getByText('Source artwork is not bundled', { exact: true })).toHaveCount(0);
  await expect(page.locator('.research-tile')).toHaveCount(0);

  await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill('B0526');
  await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
  const ground = page.locator('.terrain-art');
  await expect(ground).toBeVisible();
  await expect(page.locator('.geography-inspector')).toContainText('desert');
  await expect(page.locator('.geography-inspector')).toContainText('provisional');
  // Decode the ground tile under selection; compare actual pixels at that cell
  // before/after camera moves rather than depending on per-hex SVG nodes.
  async function groundPixel() {
    return page.evaluate(async () => {
      const selected = document.querySelector('.geography-selected')!.getBoundingClientRect();
      const x = selected.x + selected.width / 2, y = selected.y + selected.height / 2;
      for (const tile of document.querySelectorAll<SVGImageElement>('.terrain-art-tile')) {
        const box = tile.getBoundingClientRect();
        if (x >= box.x && x < box.right && y >= box.y && y < box.bottom) {
          const image = new Image();
          image.src = tile.href.baseVal;
          await image.decode();
          const canvas = document.createElement('canvas');
          canvas.width = image.naturalWidth; canvas.height = image.naturalHeight;
          const context = canvas.getContext('2d')!;
          context.drawImage(image, 0, 0);
          return [...context.getImageData(Math.floor((x - box.x) / box.width * canvas.width),
            Math.floor((y - box.y) / box.height * canvas.height), 1, 1).data];
        }
      }
      throw new Error('No original terrain tile covers the selected hex');
    });
  }
  const desertPixel = await groundPixel();
  expect(desertPixel).not.toBeNull();
  expect(desertPixel).not.toEqual([216, 220, 223, 255]);
  expect(await page.locator('.terrain-art-tile').count()).toBeLessThanOrEqual(100);

  await art.click();
  await expect(ground).toBeHidden();
  await expect(page.locator('.geography-selected')).toHaveCount(1);
  await art.click();
  await expect(ground).toBeVisible();
  await page.locator('.geography-details > summary').click();
  await page.locator('.research-viewport').focus();
  // Cross the buffered viewport boundary, then return to the initial camera.
  const steps = Math.ceil(page.viewportSize()!.width / 200) + 1;
  const selectedBeforePan = (await page.locator('.geography-selected').boundingBox())!;
  await page.keyboard.press('ArrowRight');
  const selectedAfterPan = (await page.locator('.geography-selected').boundingBox())!;
  expect(selectedAfterPan.x).toBeCloseTo(selectedBeforePan.x - 100, 1);
  expect(await groundPixel()).toEqual(desertPixel);
  for (let i = 1; i < steps; i++) await page.keyboard.press('ArrowRight');
  for (let i = 0; i < steps; i++) await page.keyboard.press('ArrowLeft');
  expect(await groundPixel()).toEqual(desertPixel);
  await page.keyboard.press('+');
  expect(await groundPixel()).not.toEqual([216, 220, 223, 255]);
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.research-board')).toHaveAttribute('data-source-visible', 'false');
  // A distant jump must refresh terrain outside the previous viewport buffer.
  await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill('B5614');
  await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
  await expect(page.locator('.geography-inspector')).toContainText('mountain');
  expect(await groundPixel()).not.toEqual(desertPixel);
  expect(await groundPixel()).not.toEqual([216, 220, 223, 255]);
  expect(failedArt).toEqual([]);
  expect(errors).toEqual([]);
});

test('source scan and original terrain can be compared together or independently', async ({ page }) => {
  const levels = Array.from({ length: 6 }, (_, z) => {
    const width = Math.ceil(14310 / 2 ** (5 - z));
    const height = Math.ceil(4632 / 2 ** (5 - z));
    return { z, width, height, columns: Math.ceil(width / 512), rows: Math.ceil(height / 512) };
  });
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({
    json: { board_width: 14310, board_height: 4632, tile_size: 512, levels },
  }));
  await page.route(/\/tiles\/map\/\d+\/\d+_\d+\.png$/, route => route.fulfill({
    contentType: 'image/svg+xml',
    body: '<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512"><rect width="512" height="512" fill="#417ea3"/></svg>',
  }));
  await page.goto('/');
  const art = page.getByRole('button', { name: 'Original terrain', exact: true });
  const source = page.getByRole('button', { name: 'Source scan', exact: true });
  const ground = page.locator('.terrain-art-board');
  const camera = () => page.locator('.research-viewport').evaluate((el: HTMLElement) =>
    [el.dataset.centerX, el.dataset.centerY, el.dataset.zoom]);
  await expect(source).toBeEnabled();
  const initialCamera = await camera();
  await page.locator('.geography-legend > summary').click();
  const legend = page.getByRole('region', { name: 'Map legend', exact: true });
  await expect(legend.getByRole('heading', { name: 'Source scan', exact: true })).toBeHidden();
  await source.click();
  await expect(legend.getByRole('heading', { name: 'Source scan', exact: true })).toBeVisible();
  await expect(legend.getByText(/55% opacity/)).toBeVisible();
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(ground).toBeVisible();
  await expect(ground).toHaveCSS('opacity', '0.55');
  await expect(page.locator('.research-overview')).toBeVisible();
  await expect(page.locator('.research-tile[data-ready=true]').first()).toBeVisible();
  await art.click();
  await expect(ground).toBeHidden();
  await expect(legend.getByText(/55% opacity/)).toBeHidden();
  await expect(legend.getByRole('heading', { name: 'Terrain tiles', exact: true })).toBeHidden();
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.research-overview')).toBeVisible();
  await art.click();
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(ground).toBeVisible();
  await source.click();
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(legend.getByRole('heading', { name: 'Source scan', exact: true })).toBeHidden();
  await expect(ground).toHaveCSS('opacity', '1');
  await expect(page.locator('.research-tile')).toHaveCount(0);
  expect(await camera()).toEqual(initialCamera);
});

test('source-backed unfinished routes are visible without becoming completed links', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'No private scan' }));
  await page.goto('/');
  await page.getByRole('button', { name: 'Fit full board', exact: true }).click();
  // Railroad controls stay separate from the newly recognized hollow road
  // capsules. Generic unresolved ink (including blue bars) is not construction.
  await expect(page.locator('.geography-construction.unfinished-railroad')).toHaveCount(39);
  await expect(page.locator('.geography-construction[data-from="C3019"][data-to="C3020"]')).toHaveCount(0);
  await expect(page.locator('.geography-link.unfinished-railroad, .geography-link.unfinished-road')).toHaveCount(0);
  for (const [hex, to, kind, label] of [
    ['C4217', 'C4218', 'unfinished-railroad', 'Unfinished railroad'],
    ['D3602', 'D3603', 'unfinished-railroad', 'Unfinished railroad'],
    ['D3611', 'D3711', 'unfinished-railroad', 'Unfinished railroad'],
    ['C3823', 'C3824', 'unfinished-road', 'Unfinished road'],
    ['C0127', 'C0227', 'unfinished-road', 'Unfinished road'],
    ['C1331', 'C1332', 'unfinished-road', 'Unfinished road'],
    ['D3512', 'D3612', 'unfinished-road', 'Unfinished road'],
  ]) {
    await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill(hex);
    await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
    // Independent manual and extracted observations may describe the same side.
    const path = page.locator(`.geography-construction.${kind}[data-from="${hex}"][data-to="${to}"]`).first();
    // Horizontal SVG strokes have zero-height geometric bounds, so Playwright's
    // visibility predicate rejects them even when the stroke is painted.
    await expect(path).toBeAttached();
    expect(await path.evaluate((node: SVGPathElement) => node.getTotalLength())).toBeGreaterThan(80);
    await expect(path).toHaveCSS('visibility', 'visible');
    await expect(path).toHaveCSS('stroke', 'rgb(113, 68, 160)');
    await expect(path.locator('title')).toHaveText(`${label} depiction · unresolved · schematic, not an operational link`);
    const details = page.locator('.geography-inspector details').filter({ hasText: `${label} depiction — schematic` }).first();
    await expect(details).toContainText('unresolved');
    await expect(details).toContainText('not an operational link');
  }
  expect(errors).toEqual([]);
});
