import { expect, test } from '@playwright/test';

test('production assets and reference requests stay under the deployment base', async ({ page }) => {
  const base = process.env.CNA_PUBLIC_BASE || '/';
  const escaped: string[] = [];
  const failures: string[] = [];
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('request', request => {
    const url = new URL(request.url());
    if (url.hostname === '127.0.0.1' && !url.pathname.startsWith(base)) escaped.push(url.pathname);
  });
  page.on('response', response => {
    const path = new URL(response.url()).pathname;
    if (response.status() >= 400 && !path.startsWith(`${base}__source-tokens/`)
      && path !== `${base}tiles/map/manifest.json`) failures.push(`${response.status()} ${path}`);
  });
  await page.goto(base);
  await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeVisible();
  await expect(page.locator('.terrain-art image').first()).toBeVisible();
  const ground = page.locator('.terrain-art image').first();
  const groundUrl = await ground.getAttribute('href');
  const groundResponse = await page.request.get(new URL(groundUrl!, page.url()).href);
  expect(groundResponse.status()).toBe(200);
  expect(groundResponse.headers()['content-type']).toContain('image/webp');

  await page.getByRole('button', { name: 'Reinforcements', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Find an identity' }).fill('2nd Bays');
  await expect.poll(() => page.locator('.deployment-card img').first().evaluate(
    (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
  )).toBe(true);
  await page.getByRole('button', { name: 'Add planning counter', exact: true }).click();
  await page.keyboard.press('Enter');
  await expect(page.locator('.counter-marker')).toHaveCount(1);
  await expect.poll(() => page.locator('.counter-marker img').evaluate(
    (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
  )).toBe(true);

  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  await expect.poll(() => page.locator('.catalog-type img').first().evaluate(
    (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
  )).toBe(true);
  await page.getByRole('searchbox', { name: 'Search records' }).fill('1st armored div hq');
  await page.getByRole('button', { name: 'Explore Headquarters', exact: true }).click();
  const referenceResponse = page.waitForResponse(response => response.url().endsWith(`${base}data/unit-reference.json`));
  await page.locator('[data-action="catalog-record"]').first().click();
  const references = await referenceResponse;
  expect(references.status()).toBe(200);
  await expect(page.locator('.catalog-original-tokens')).toContainText('private-review');
  expect(escaped).toEqual([]);
  expect(failures).toEqual([]);
  expect(errors).toEqual([]);
});
