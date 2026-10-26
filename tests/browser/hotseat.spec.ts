import { expect, test, type Page } from '@playwright/test';

async function add(page: Page, identity: string) {
  await page.getByRole('button', { name: 'Reinforcements', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Find an identity' }).fill(identity);
  await page.getByRole('button', { name: 'Add planning counter', exact: true }).click();
  await page.keyboard.press('Enter');
}

async function note(page: Page, value: string) {
  await page.getByLabel('Rulebook adjudication and costs').fill(value);
}

async function endPortion(page: Page) {
  await page.getByRole('button', { name: 'Complete player portion / advance stage', exact: true }).click();
  await note(page, 'Test players adjudicated applicable phases using the rulebook.');
  await page.getByRole('button', { name: 'Confirm player portion complete', exact: true }).click();
}

test('hot-seat setup, orders, opposing turns, resume and concession work on the real board', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('/');
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await add(page, 'unit-commonwealth-2nd-bays');
  await add(page, 'unit-italian-vii-m-tank-bn');
  await page.getByRole('button', { name: 'Play / Save', exact: true }).click();
  await page.getByLabel('Operation / setup reference').fill('Browser integration fixture, not a historical setup');
  await page.getByRole('button', { name: 'Start hot-seat game', exact: true }).click();
  await expect(page.locator('.calendar-chip')).toContainText('Axis to act');
  await page.locator('#board-roster [data-counter="1"]').click();
  await expect(page.locator('#board-detail')).toContainText('Only the active player');
  await page.locator('#board-roster [data-counter="2"]').click();
  await page.getByRole('button', { name: 'Supply ledger', exact: true }).click();
  await page.getByLabel('Fuel points', { exact: true }).fill('7/2');
  await page.getByLabel('Ammunition points', { exact: true }).fill('3');
  await page.getByLabel('Stores points', { exact: true }).fill('1');
  await page.getByLabel('Water points', { exact: true }).fill('5');
  await note(page, 'Test ledger, player-entered balances');
  await page.getByRole('button', { name: 'Record order', exact: true }).click();
  await expect(page.locator('#board-detail')).toContainText('Fuel 7/2');
  await page.getByRole('button', { name: 'Move', exact: true }).click();
  await page.getByLabel('Destination hex').fill('C4021');
  await page.getByLabel('Adjudicated fuel cost (optional)').fill('4');
  await note(page, 'Adjudicated route and cost for integration test');
  await page.getByRole('button', { name: 'Record order', exact: true }).click();
  await expect(page.getByRole('alert')).toHaveText('Insufficient fuel for this adjudicated move.');
  await page.getByLabel('Adjudicated fuel cost (optional)').fill('2/3');
  await page.getByRole('button', { name: 'Record order', exact: true }).click();
  await expect(page.locator('#board-detail')).toContainText('C4021');
  await expect(page.locator('#board-detail')).toContainText('Fuel 17/6');
  await page.getByRole('button', { name: 'Construction', exact: true }).click();
  await note(page, 'Construction completion adjudicated');
  await page.getByRole('button', { name: 'Record order', exact: true }).click();
  await expect(page.locator('#board-detail')).toContainText('Construction recorded');
  await page.getByRole('button', { name: 'Play / Save', exact: true }).click();
  await page.getByRole('button', { name: 'Roll D6', exact: true }).click();
  await expect(page.locator('#board-message')).toContainText('D6 =');
  const die = await page.locator('#board-message').innerText();
  await endPortion(page);
  await expect(page.locator('.calendar-chip')).toContainText('Commonwealth to act');
  await endPortion(page);
  await expect(page.locator('.calendar-chip')).toContainText('Stage complete');
  await page.getByRole('button', { name: 'Complete player portion / advance stage', exact: true }).click();
  await page.getByLabel('First player (adjudicated initiative)').selectOption('commonwealth');
  await note(page, 'Initiative and preliminary phases adjudicated');
  await page.getByRole('button', { name: 'Begin next stage', exact: true }).click();
  await expect(page.locator('.calendar-chip')).toContainText('Stage 2 of 3');
  await page.reload();
  await expect(page.locator('.calendar-chip')).toContainText('Commonwealth to act');
  await page.getByRole('button', { name: 'Play / Save', exact: true }).click();
  await expect(page.locator('.play-log')).toContainText(die);
  await page.getByText('Save / replace this game', { exact: true }).click();
  const save = await page.getByLabel('Session JSON').inputValue();
  await page.getByLabel('Session JSON').fill('{"schema":999}');
  await page.getByRole('button', { name: 'Replace with imported game', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Invalid rulebook session save');
  await page.getByLabel('Session JSON').fill(save);
  await page.getByRole('button', { name: 'Replace with imported game', exact: true }).click();
  await page.locator('#board-roster [data-counter="1"]').click();
  await page.getByRole('button', { name: 'Combat result', exact: true }).click();
  await page.getByLabel('Defender eliminated', { exact: true }).check();
  await note(page, 'Whole-counter elimination adjudicated from rulebook');
  await page.getByRole('button', { name: 'Record order', exact: true }).click();
  await expect(page.locator('#board-roster [data-counter="2"]')).toContainText('Eliminated');
  await expect(page.locator('.counter-marker[data-counter="2"]')).toHaveCount(0);
  await endPortion(page);
  await page.getByRole('button', { name: 'Play / Save', exact: true }).click();
  await page.getByRole('button', { name: 'Concede game', exact: true }).click();
  await page.getByRole('button', { name: 'Confirm concession', exact: true }).click();
  await expect(page.locator('.calendar-chip')).toContainText('Commonwealth wins by concession');
  await page.reload();
  await expect(page.locator('.calendar-chip')).toContainText('Commonwealth wins by concession');
  expect(errors).toEqual([]);
});
