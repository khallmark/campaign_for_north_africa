import { expect, test } from '@playwright/test';

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
  await source.click();
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(ground).toBeVisible();
  await expect(ground).toHaveCSS('opacity', '0.55');
  await expect(page.locator('.research-overview')).toBeVisible();
  await expect(page.locator('.research-tile[data-ready=true]').first()).toBeVisible();
  await art.click();
  await expect(ground).toBeHidden();
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.research-overview')).toBeVisible();
  await art.click();
  await expect(source).toHaveAttribute('aria-pressed', 'true');
  await expect(ground).toBeVisible();
  await source.click();
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(ground).toHaveCSS('opacity', '1');
  await expect(page.locator('.research-tile')).toHaveCount(0);
  expect(await camera()).toEqual(initialCamera);
});
