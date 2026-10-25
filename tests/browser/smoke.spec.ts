import { expect, test, type Page } from '@playwright/test';

async function addCounter(page: Page) {
  await page.getByRole('button', { name: 'Reinforcements', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Find an identity' }).fill('2nd Bays');
  await page.getByRole('button', { name: 'Add planning counter', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page.locator('.counter-marker')).toHaveCount(1);
}

test('full-board workspace removes the scenario and restores modal keyboard focus', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeVisible();
  await expect(page.locator('header, .battle-map')).toHaveCount(0);
  await expect(page.locator('#board-roster')).toContainText('0 planning counters');
  const bounds = await page.locator('.research-viewport').boundingBox();
  expect(bounds).toEqual({ x: 0, y: 0, ...page.viewportSize()! });
  for (const name of ['Field manual', 'Unit catalog', 'Reinforcements', 'New operation']) {
    const opener = page.getByRole('button', { name, exact: true });
    await opener.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByRole('dialog', { name, exact: true })).toBeVisible();
    await page.keyboard.press('Tab');
    expect(await page.evaluate(() => !!document.activeElement?.closest('dialog'))).toBe(true);
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
    await expect(opener).toBeFocused();
  }
  expect(errors).toEqual([]);
});

test('calendar exposes 111 weekly turns and three reference stages without executing a turn', async ({ page }) => {
  await page.goto('/');
  const calendar = page.getByRole('button', { name: 'Open campaign calendar' });
  await calendar.click();
  await expect(page.getByRole('dialog')).toContainText('111 Game-Turns, not 111 days.');
  await expect(page.locator('[data-action=choose-turn]')).toHaveCount(111);
  await expect(page.locator('[data-action=choose-stage]')).toHaveCount(3);
  await page.getByRole('button', { name: 'Game-Turn 63: January I, 1942', exact: true }).click();
  await page.getByRole('button', { name: 'Stage 3', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(calendar).toBeFocused();
  await expect(calendar).toContainText('January I, 1942');
  await expect(calendar).toContainText('Stage 3 of 3');
  await page.getByRole('button', { name: 'End stage — unavailable in review mode' }).click();
  await expect(page.getByRole('dialog')).toContainText('No active simulation');
  await page.getByRole('button', { name: 'Close dialog' }).click();
  await expect(calendar).toContainText('Week / turn 63 of 111');
  await expect(calendar).toContainText('Stage 3 of 3');
});

test('planning selection and icon actions track pan and zoom; hover waits one second', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await addCounter(page);
  await expect(page.locator('#board-detail')).toContainText('2nd Bays');
  await expect(page.locator('#board-detail')).toContainText('User-placed');
  const marker = page.locator('.counter-marker');
  const actions = page.getByRole('group', { name: 'Selected unit actions' });
  const initial = (await marker.boundingBox())!;
  const initialActions = (await actions.boundingBox())!;
  expect(initialActions.y).toBeGreaterThan(initial.y + initial.height);
  expect(initialActions.x + initialActions.width / 2).toBeCloseTo(initial.x + initial.width / 2, 2);
  const { width, height } = page.viewportSize()!;
  await page.mouse.move(width / 2 + 180, height * .7);
  await page.mouse.down();
  await page.mouse.move(width / 2 + 300, height * .7 - 47, { steps: 10 });
  await page.mouse.up();
  const moved = (await marker.boundingBox())!;
  const movedActions = (await actions.boundingBox())!;
  expect(moved.x - initial.x).toBeCloseTo(120, 1);
  expect(moved.y - initial.y).toBeCloseTo(-47, 1);
  expect(movedActions.x - initialActions.x).toBeCloseTo(120, 1);
  expect(movedActions.y - initialActions.y).toBeCloseTo(-47, 1);
  await page.getByRole('button', { name: 'Zoom in map', exact: true }).click();
  await expect(page.locator('.research-viewport')).toHaveAttribute('data-zoom', '1.25');
  const zoomed = (await marker.boundingBox())!;
  expect(zoomed.x + 32 - width / 2).toBeCloseTo((moved.x + 32 - width / 2) * 1.25, 1);
  expect(zoomed.y + 32 - height / 2).toBeCloseTo((moved.y + 32 - height / 2) * 1.25, 1);
  await page.getByRole('button', { name: 'Locate selected unit' }).click();
  const move = page.getByRole('button', { name: 'Reposition planning counter', exact: true });
  const tooltip = page.getByRole('tooltip', { name: 'Reposition planning counter', exact: true });
  await move.hover();
  await page.waitForTimeout(700);
  await expect(tooltip).toBeHidden();
  await expect(tooltip).toBeVisible({ timeout: 800 });
  await page.mouse.move(20, height - 100);
  await expect(tooltip).toBeHidden();
  await marker.focus();
  await page.keyboard.press('Tab');
  await expect(move).toBeFocused();
  await expect(tooltip).toBeVisible();
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Enter');
  await expect(page.locator('#board-placement')).toBeHidden();
  await page.getByRole('button', { name: 'Attack — requires an active simulation' }).click();
  await expect(page.getByRole('dialog')).toContainText('No active simulation');
  await page.keyboard.press('Escape');
  await expect(page.getByRole('button', { name: 'Attack — requires an active simulation' })).toBeFocused();
  await page.getByRole('button', { name: 'Remove planning counter' }).click();
  await expect(marker).toHaveCount(0);
  await expect(page.locator('#board-roster')).toContainText('0 planning counters');
  expect(errors).toEqual([]);
});

test('narrow layout keeps floating controls and dialogs reachable without horizontal scroll', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await addCounter(page);
  for (const selector of ['.calendar-chip', '.board-tools', '.order-panel', '.unit-panel', '.research-controls', '.counter-actions']) {
    const box = (await page.locator(selector).boundingBox())!;
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
    expect(box.y + box.height).toBeLessThanOrEqual(844);
  }
  await page.locator('.order-panel > summary').click();
  await page.locator('.unit-panel > summary').click();
  await expect(page.locator('#board-detail')).toBeHidden();
  await page.getByRole('button', { name: 'Open campaign calendar' }).click();
  await page.getByRole('button', { name: 'Game-Turn 111: January I, 1943', exact: true }).click();
  await page.keyboard.press('Escape');
  await expect(page.locator('.calendar-chip')).toContainText('January I, 1943');
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
  await page.getByRole('button', { name: 'New operation', exact: true }).click();
  await page.getByRole('button', { name: 'Keep this board' }).click();
  await expect(page.locator('.counter-marker')).toHaveCount(1);
  await page.getByRole('button', { name: 'New operation', exact: true }).click();
  await page.getByRole('button', { name: 'Clear review board' }).click();
  await expect(page.locator('.counter-marker')).toHaveCount(0);
});
