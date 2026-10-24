import { expect, test } from '@playwright/test';

test('sandbox starts without browser errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() === 'error') errors.push(message.text());
  });

  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'The road to Tobruk' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Battlefield' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'End stage', exact: true })).toBeEnabled();
  await expect(page.getByText('Loading Field Command…')).toHaveCount(0);
  await page.evaluate(() => document.fonts.ready);
  expect(errors).toEqual([]);
});
