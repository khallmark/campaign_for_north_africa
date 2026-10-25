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
  await expect(page.getByRole('button', { name: 'Source scan', exact: true })).toBeDisabled();
  await expect(page.locator('.research-status')).toContainText('Original terrain art');
  await expect(page.getByText('Source artwork is not bundled', { exact: true })).toHaveCount(0);
  await expect(page.locator('.research-tile')).toHaveCount(0);

  await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill('B0526');
  await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
  const desert = page.locator('[data-hex="B0526"][data-terrain="desert"], [data-hex="B0526"] [data-terrain="desert"]');
  await expect(desert).toBeVisible();
  await expect(page.locator('.geography-inspector')).toContainText('provisional');
  await expect(page.locator('.terrain-art-unknown').first()).toHaveAttribute('fill', '#d8dcdf');
  // Inspect the real browser's image decoding, not only SVG element existence.
  expect(await page.evaluate(async () => {
    const image = new Image();
    image.src = '/art/terrain-v1/surfaces/desert.png';
    await image.decode();
    return [image.naturalWidth, image.naturalHeight];
  })).toEqual([402, 402]);

  await art.click();
  await expect(page.locator('.terrain-art')).toBeHidden();
  await expect(page.locator('.geography-selected')).toHaveCount(1);
  await art.click();
  await expect(desert).toBeVisible();
  await page.locator('.geography-details > summary').click();
  await page.locator('.research-viewport').focus();
  await page.keyboard.press('+');
  await page.keyboard.press('ArrowRight');
  await expect(desert).toBeVisible();
  const texture = (await desert.boundingBox())!;
  const selected = (await page.locator('.geography-selected').boundingBox())!;
  expect(texture.x + texture.width / 2).toBeCloseTo(selected.x + selected.width / 2, 1);
  expect(texture.y + texture.height / 2).toBeCloseTo(selected.y + selected.height / 2, 1);
  await expect(art).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.research-board')).toHaveAttribute('data-source-visible', 'false');
  expect(failedArt).toEqual([]);
  expect(errors).toEqual([]);
});
