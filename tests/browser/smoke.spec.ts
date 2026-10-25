import { expect, test, type Page } from '@playwright/test';

test('startup paints without waiting for WebAssembly or external fonts', async ({ page }) => {
  let release!: () => void;
  const pending = new Promise<void>(resolve => { release = resolve; });
  await page.route('**/*.wasm', async route => { await pending; await route.continue(); });
  await page.route('https://fonts.googleapis.com/**', async route => {
    await pending;
    await route.fulfill({ contentType: 'text/css', body: '' });
  });
  try {
    await page.goto('/', { waitUntil: 'commit' });
    await expect(page.locator('.startup-status')).toBeVisible();
    await expect(page.locator('.startup-status')).toContainText('Loading the map workspace');
    await expect(page.locator('.startup')).toHaveCSS('position', 'fixed');
    await expect.poll(() => page.evaluate(() => performance.getEntriesByName('first-contentful-paint').length)).toBe(1);
    await expect(page.locator('.research-viewport')).toHaveCount(0);
    release();
    await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeVisible();
    await expect(page.locator('.startup')).toHaveCount(0);
  } finally {
    release();
  }
});

function collectErrors(page: Page) {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => {
    if (message.type() !== 'error') return;
    // Production intentionally excludes private source tiles. Only this exact
    // missing-manifest response is expected; application console errors still fail.
    if (new URL(message.location().url || 'about:blank').pathname === '/tiles/map/manifest.json'
      && message.text() === 'Failed to load resource: the server responded with a status of 404 (Not Found)') return;
    errors.push(message.text());
  });
  return errors;
}

async function chooseCounter(page: Page) {
  await page.getByRole('button', { name: 'Reinforcements', exact: true }).click();
  await page.getByRole('searchbox', { name: 'Find an identity' }).fill('2nd Bays');
  await page.getByRole('button', { name: 'Add planning counter', exact: true }).click();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Full-board map', exact: true })).toBeFocused();
}

async function addCounter(page: Page) {
  await chooseCounter(page);
  await page.keyboard.press('Enter');
  await expect(page.locator('.counter-marker')).toHaveCount(1);
}

test('selecting and removing one counter leaves another of the same identity unchanged', async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await addCounter(page);
  const rows = page.locator('.board-roster-unit');
  const firstPosition = await rows.first().locator('small').innerText();
  await chooseCounter(page);
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await expect(rows).toHaveCount(2);
  await expect(page.locator('.counter-marker')).toHaveCount(2);
  const secondPosition = await rows.last().locator('small').innerText();
  expect(secondPosition).not.toBe(firstPosition);
  const secondId = (await rows.last().getAttribute('data-counter'))!;
  const secondMarker = page.locator(`.counter-marker[data-counter="${secondId}"]`);
  const secondCoordinates = await secondMarker.locator('..').evaluate(el => [el.getAttribute('data-board-x'), el.getAttribute('data-board-y')]);

  await rows.first().click();
  await expect(rows.first()).toHaveAttribute('aria-pressed', 'true');
  await expect(secondMarker).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('#board-detail .unit-facts dd').first()).toHaveText(firstPosition.split(' · ')[0]);
  await secondMarker.click();
  await expect(rows.last()).toHaveAttribute('aria-pressed', 'true');
  await expect(rows.first()).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('#board-detail .unit-facts dd').first()).toHaveText(secondPosition.split(' · ')[0]);

  // Remove the older counter, not the last-added counter or every matching identity.
  await rows.first().click();
  await page.getByRole('button', { name: 'Remove planning counter' }).click();
  await expect(rows).toHaveCount(1);
  await expect(rows).toHaveAttribute('data-counter', secondId);
  await expect(rows.locator('small')).toHaveText(secondPosition);
  await expect(page.locator('.counter-marker')).toHaveCount(1);
  expect(await secondMarker.locator('..').evaluate(el => [el.getAttribute('data-board-x'), el.getAttribute('data-board-y')])).toEqual(secondCoordinates);
  await expect(page.locator('#board-detail')).toContainText('Select a unit');
  await rows.click();
  await expect(secondMarker).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('group', { name: 'Selected unit actions' })).toHaveCount(1);
  expect(errors).toEqual([]);
});

test('off-section placement is rejected and cancellation preserves existing positions', async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  const map = page.getByRole('region', { name: 'Full-board map', exact: true });
  async function viewChart() {
    await page.getByRole('button', { name: 'Open campaign calendar' }).click();
    await page.getByText('Sequence of play and source notes', { exact: true }).click();
    await page.getByRole('button', { name: 'View original track on map' }).click();
    await map.focus();
  }
  // The original Game-Turn chart is outside the playable map sections.
  await viewChart();
  await chooseCounter(page);
  await page.keyboard.press('Enter');
  await expect(page.locator('#board-message')).toContainText('Choose a position within a mapped section');
  await expect(page.locator('#board-placement')).toBeVisible();
  await expect(page.locator('.counter-marker')).toHaveCount(0);
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('#board-placement')).toBeHidden();
  await expect(page.locator('.board-roster-unit small')).toHaveText('Awaiting placement');
  await expect(page.locator('#board-detail .unit-facts dd').first()).toHaveText('Not placed');

  // Cancel leaves the newly added identity unplaced; it can still be placed later.
  await page.getByRole('button', { name: 'Place counter', exact: true }).click();
  await page.keyboard.press('Home');
  await page.keyboard.press('Enter');
  await expect(page.locator('.counter-marker')).toHaveCount(1);
  const position = await page.locator('.board-roster-unit small').innerText();
  const coordinates = await page.locator('.board-counter').evaluate(el => [el.getAttribute('data-board-x'), el.getAttribute('data-board-y')]);
  await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
  await page.getByRole('button', { name: 'Reposition planning counter', exact: true }).click();
  await page.keyboard.press('ArrowRight');
  await page.keyboard.press('Escape');
  await expect(page.locator('#board-placement')).toBeHidden();
  await expect(page.locator('.board-roster-unit small')).toHaveText(position);
  expect(await page.locator('.board-counter').evaluate(el => [el.getAttribute('data-board-x'), el.getAttribute('data-board-y')])).toEqual(coordinates);

  await page.getByRole('button', { name: 'Reposition planning counter', exact: true }).click();
  await viewChart();
  await page.keyboard.press('Enter');
  await expect(page.locator('#board-message')).toContainText('Choose a position within a mapped section');
  await expect(page.locator('#board-placement')).toBeVisible();
  await expect(page.locator('.board-roster-unit small')).toHaveText(position);
  await page.getByRole('button', { name: 'Cancel', exact: true }).click();
  await expect(page.locator('#board-placement')).toBeHidden();
  await page.getByRole('button', { name: 'Locate selected unit' }).click();
  expect(await page.locator('.board-counter').evaluate(el => [el.getAttribute('data-board-x'), el.getAttribute('data-board-y')])).toEqual(coordinates);
  await expect(page.locator('.board-roster-unit small')).toHaveText(position);
  expect(errors).toEqual([]);
});

test('catalog controls wire search, selects and pagination into visible results', async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto('/');
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  const count = page.locator('.catalog-count');
  const search = page.getByRole('searchbox', { name: 'Search records' });
  const unitType = page.getByRole('combobox', { name: 'Unit type', exact: true });
  const nationality = page.getByRole('combobox', { name: 'Nationality', exact: true });
  await expect(count).toContainText('Unit types:');
  await expect(page.locator('.catalog-entry')).toHaveCount(0);
  const cruiser = page.getByRole('button', { name: 'Explore Anti air cruiser', exact: true });
  await cruiser.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'Anti air cruiser', exact: true })).toBeFocused();
  await expect(count).toContainText('Showing 1–3 of 3 records');
  await expect(page.locator('.catalog-list .catalog-name strong')).toHaveText(['Euryalus', 'Dido', 'Naiad']);
  for (const text of await page.locator('.catalog-list .catalog-name > span').allTextContents()) {
    expect(text).toBe('Anti air cruiser · Naval unit · Commonwealth · Allies');
  }
  const dido = page.locator('.catalog-list .catalog-entry').filter({ hasText: 'Dido' });
  await dido.click();
  await expect(page.locator('#catalog-detail table')).toBeVisible();
  await page.getByRole('button', { name: 'Back to unit types', exact: true }).click();
  await expect(cruiser).toBeFocused();
  await search.fill('Dido');
  await expect(page.locator('.catalog-type')).toHaveCount(1);
  await cruiser.click();
  await expect(count).toContainText('Showing 1–1 of 1 records');
  await page.getByRole('button', { name: 'Back to unit types', exact: true }).click();
  await expect(search).toHaveValue('Dido');
  await search.fill('');
  await page.getByRole('button', { name: 'Explore Motorized infantry', exact: true }).click();
  await search.fill('1st Rifle Brigade');
  await page.getByRole('combobox', { name: 'Record type', exact: true }).selectOption('formation');
  await page.getByRole('combobox', { name: 'Side', exact: true }).selectOption({ label: 'Allies' });
  await nationality.selectOption({ label: 'Commonwealth' });
  await expect(count).toContainText('Showing 1–1 of 1 records');
  await expect(page.locator('.catalog-list .catalog-name > span')).toHaveText('Motorized infantry · Formation · Commonwealth · Allies');
  await page.locator('.catalog-list .catalog-entry').click();
  const rifle = page.locator('#catalog-detail');
  const direct = rifle.locator('.catalog-direct-stats');
  const profile = rifle.locator('.catalog-profile-stats');
  await expect(direct).toHaveCount(0);
  await expect(rifle).not.toContainText(/unresolved/i);
  await expect(profile).toContainText('Commonwealth infantry profile l · Profile cw-l');
  await expect(profile).toContainText('not current effective CPA');
  await profile.getByText('Base profile characteristics and sources', { exact: true }).click();
  await expect(profile.locator('td strong').first()).toHaveText('10+');
  await expect(profile).toContainText('Do not treat base as motorized allowance');
  await expect(profile).toContainText('Source dash');
  await expect(profile.locator('details')).toContainText('PDF p. 133');
  await page.getByRole('button', { name: 'Back to Motorized infantry', exact: true }).click();
  await nationality.selectOption({ label: 'German' });
  await expect(count).toContainText('No matching records');
  await nationality.selectOption('commonwealth');
  await page.getByRole('combobox', { name: 'Side', exact: true }).selectOption('axis');
  await expect(count).toContainText('No matching records');
  await page.getByRole('combobox', { name: 'Side', exact: true }).selectOption('allied');
  await unitType.selectOption('infantry');
  await expect(count).toContainText('No matching records');
  await search.fill('');
  await expect(count).toContainText('Showing 1–24');
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  await expect(count).toContainText('Showing 25–48');
  await expect(count).toBeFocused();
  const metadata = page.locator('.catalog-list .catalog-name > span');
  expect(await metadata.count()).toBe(24);
  for (const text of await metadata.allTextContents()) {
    expect(text).toBe('Infantry · Formation · Commonwealth · Allies');
  }
  await unitType.selectOption('headquarters');
  await expect(count).toContainText('Showing 1–');
  for (const text of await metadata.allTextContents()) {
    expect(text).toBe('Headquarters · Formation · Commonwealth · Allies');
  }
  await page.getByRole('combobox', { name: 'Record type', exact: true }).selectOption('equipment');
  await expect(count).toContainText('No matching records');
  expect(errors).toEqual([]);
});

test('anti-aircraft profile dashes remain distinct from equipment ratings', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  const search = page.getByRole('searchbox', { name: 'Search records' });
  const kind = page.getByRole('combobox', { name: 'Record type', exact: true });
  await page.getByRole('combobox', { name: 'Unit type', exact: true }).selectOption('anti-air');
  await search.fill('13th Light AA Regt');
  await kind.selectOption('formation');
  await page.locator('.catalog-list .catalog-entry').click();
  const entry = page.locator('#catalog-detail');
  const equipment = entry.locator('.catalog-equipment-ratings');
  await expect(equipment).toContainText('not an assigned loadout');
  await expect(equipment.getByRole('row', { name: /Light AA \(Bofors 40mm\)/ }).locator('td strong')).toHaveText('1');
  await expect(equipment.getByRole('row', { name: /Heavy AA \(3.7-inch\)/ }).locator('td strong')).toHaveText('4');
  await expect(equipment).toContainText('Current TOE and weapon mix are required for a unit total');
  await entry.getByText('Base profile characteristics and sources', { exact: true }).click();
  const rating = entry.locator('.catalog-profile-stats').getByRole('row').filter({ has: page.getByRole('rowheader', { name: /^Anti-aircraft rating/ }) });
  await expect(rating.locator('td')).toContainText('Source dash');
  await expect(rating.locator('td')).not.toContainText('Not applicable');
  await expect(rating).toContainText('Weapon mix or attachments may supply ratings');
  await page.getByRole('button', { name: 'Back to Anti air', exact: true }).click();
  await search.fill('Bofors');
  await kind.selectOption('equipment');
  await page.locator('.catalog-list .catalog-entry').click();
  const weaponRating = entry.getByRole('row').filter({ has: page.getByRole('rowheader', { name: /^Anti-aircraft rating/ }) });
  await expect(weaponRating.locator('td strong')).toHaveText('1');
  await expect(weaponRating.locator('td')).toContainText('rating per TOE point');
  await expect(weaponRating).toContainText('PDF p. 138');
});

test('formation references distinguish base CPA, arrival totals, conditional and missing loadouts', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  const detail = page.locator('#catalog-detail');
  async function open(id: string, type: string) {
    if (await detail.isVisible()) await page.getByRole('button', { name: 'Back to unit types', exact: true }).click();
    await page.getByRole('combobox', { name: 'Unit type', exact: true }).selectOption(type);
    await page.getByRole('searchbox', { name: 'Search records' }).fill(id);
    await page.locator('.catalog-list .catalog-entry').click();
  }
  await open('oa-149-084-60th-machinegun-bn', 'heavy-weapons');
  await expect(detail.locator('.catalog-base-cpa > strong')).toHaveText('8');
  await expect(detail.locator('.catalog-base-cpa')).toContainText('CP');
  await expect(detail.locator('.catalog-arrival-facts > div').nth(1).locator('strong')).toHaveText('2');
  await expect(detail.locator('.catalog-arrival-facts > div').nth(2).locator('strong')).toHaveText('-1');
  await expect(detail.locator('.catalog-arrival')).toContainText('1/12');
  await expect(detail.locator('.catalog-direct-stats')).toHaveCount(0);
  await expect(detail).not.toContainText(/unresolved/i);
  await expect(detail).not.toContainText('not transcribed');
  const totals = detail.locator('.catalog-arrival-totals');
  const aa = totals.getByRole('row', { name: /^Anti-aircraft rating/ });
  await open('oa-124-002-13th-light-aa-regt', 'anti-air');
  await expect(aa.locator('td strong')).toHaveText('6');
  await expect(aa.locator('td')).toContainText('Actual AA points');
  await expect(aa.locator('td')).toContainText('6 × 1');
  await detail.locator('.catalog-component summary').click();
  await expect(detail.locator('.catalog-component')).toContainText('weapons-cw-light-aa-bofors-40mm');
  await expect(detail.locator('.catalog-component').getByRole('row', { name: /^Anti-aircraft rating/ }).locator('td strong')).toHaveText('1');
  await open('oa-117-024-102nd-nh-light-anti-aircraft-anti-tank-regt', 'anti-tank-anti-air');
  await expect(aa.locator('td strong')).toHaveText('2');
  const antiArmor = totals.getByRole('row', { name: /^Anti-armor/ });
  await expect(antiArmor.locator('td strong')).toHaveCount(0);
  await expect(antiArmor.locator('td')).toContainText('Conditional');
  await open('oa-116-003-2nd-bays', 'tank');
  const assault = totals.getByRole('row', { name: /^Offensive close assault/ });
  await expect(assault.locator('td strong')).toHaveText('28');
  await expect(assault.locator('td')).toContainText('Raw combat points (before ÷10)');
  await open('oa-165-065-gruppe-badde', 'headquarters');
  await expect(totals.locator('td strong')).toHaveCount(0);
  await expect(detail.locator('.catalog-arrival')).toContainText('That of the attached units');
  await expect(detail.locator('.catalog-arrival-components')).toContainText('A profile maximum is not an assignment');
  await open('oa-123-096-11th-royal-horse-arty-regt-bishop-refit-counter', 'artillery');
  await expect(detail.locator('.catalog-arrival')).toContainText(/optional/i);
  await expect(totals.locator('td strong')).toHaveCount(0);
  await page.setViewportSize({ width: 390, height: 844 });
  expect(await detail.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
});

test('catalog breadcrumbs preserve the dialog, filters, page and focus through animated navigation', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Unit catalog', exact: true });
  const bounds = await dialog.boundingBox();
  const crumbs = page.getByRole('navigation', { name: 'Catalog breadcrumb' });
  await expect(crumbs.locator('[aria-current="page"]')).toHaveText('Unit types');
  await page.getByRole('button', { name: 'Explore Infantry', exact: true }).click();
  await page.getByRole('combobox', { name: 'Nationality', exact: true }).selectOption('commonwealth');
  await page.getByRole('combobox', { name: 'Record type', exact: true }).selectOption('formation');
  await page.getByRole('button', { name: 'Next', exact: true }).click();
  const card = page.locator('.catalog-list [data-action="catalog-record"]').nth(17);
  const id = await card.getAttribute('id');
  const name = await card.locator('.catalog-name strong').textContent();
  await card.scrollIntoViewIfNeeded();
  await card.focus();
  const scroll = await page.locator('#catalog-browser').evaluate(el => el.scrollTop);
  await page.keyboard.press('Enter');
  await expect(crumbs.locator('[aria-current="page"]')).toHaveText(name!);
  await expect(page.locator('#catalog-detail .catalog-record-heading')).toBeFocused();
  await expect(page.locator('#catalog-detail .catalog-record')).toBeVisible();
  expect(await dialog.boundingBox()).toEqual(bounds);
  await expect(page.locator('#catalog-detail')).toHaveCSS('animation-name', 'catalog-enter');
  await crumbs.getByRole('button', { name: 'Back to Infantry', exact: true }).click();
  await expect(page.locator(`[id="${id}"]`)).toBeFocused();
  await expect(page.locator('.catalog-count')).toContainText('Showing 25–48');
  await expect(page.getByRole('combobox', { name: 'Nationality', exact: true })).toHaveValue('commonwealth');
  expect(await page.locator('#catalog-browser').evaluate(el => el.scrollTop)).toEqual(scroll);
  expect(await dialog.boundingBox()).toEqual(bounds);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.locator(`[id="${id}"]`).click();
  await expect(page.locator('#catalog-detail')).toHaveCSS('animation-name', 'none');
  await crumbs.getByRole('button', { name: 'Back to unit types', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Explore Infantry', exact: true })).toBeFocused();
  expect(await dialog.boundingBox()).toEqual(bounds);
  await page.getByRole('searchbox', { name: 'Search records' }).fill('no-such-unit');
  await expect(page.locator('.catalog-count')).toContainText('No matching');
  expect(await dialog.boundingBox()).toEqual(bounds);
  await page.setViewportSize({ width: 390, height: 844 });
  const narrowBounds = await dialog.boundingBox();
  await page.getByRole('searchbox', { name: 'Search records' }).fill('13th Light AA Regt');
  await page.getByRole('button', { name: 'Explore Anti air', exact: true }).click();
  expect(await dialog.boundingBox()).toEqual(narrowBounds);
  await page.locator('.catalog-list [data-action="catalog-record"]').click();
  await expect(crumbs.locator('[aria-current="page"]')).toHaveText('13th Light AA Regt');
  expect(await dialog.boundingBox()).toEqual(narrowBounds);
  expect(await page.locator('#catalog-detail').evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Unit catalog', exact: true })).toBeFocused();
});

test('catalog original tokens follow reviewed links and survive late responses and unavailable images', async ({ page }) => {
  let release!: () => void;
  const pending = new Promise<void>(resolve => { release = resolve; });
  await page.route('**/data/unit-reference.json', async route => {
    const response = await route.fetch();
    await pending;
    await route.fulfill({ response });
  });
  let unavailable = false;
  await page.route('**/__source-tokens/**', route => route.fulfill(unavailable
    ? { status: 404, body: 'Unavailable' }
    : { contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg" width="60" height="60"><rect width="60" height="60" fill="olive"/></svg>' }));
  await page.goto('/');
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  const search = page.getByRole('searchbox', { name: 'Search records' });
  const type = page.getByRole('combobox', { name: 'Unit type', exact: true });
  await type.selectOption('headquarters');
  await search.fill('oa-116-001-1st-armored-div-hq');
  await page.locator('.catalog-list .catalog-card').click();
  const tokens = page.locator('#catalog-detail .catalog-original-tokens');
  await expect(tokens).toContainText('Loading original-token references');
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await type.selectOption('anti-air');
  await search.fill('13th Light AA Regt');
  await page.locator('.catalog-list .catalog-card').click();
  release();
  await expect(tokens).toContainText('No reviewed original-token link');
  await expect(tokens.locator('img')).toHaveCount(0);
  await page.getByRole('button', { name: 'Back', exact: true }).click();
  await type.selectOption('headquarters');
  await search.fill('oa-116-001-1st-armored-div-hq');
  await page.locator('.catalog-list .catalog-card').click();
  await expect(tokens.locator('img')).toHaveCount(1);
  await expect(tokens.locator('img')).toHaveAttribute('src', '/__source-tokens/BR%201%20-%20none.svg');
  await expect(tokens.locator('[data-state="loaded"]')).toBeVisible();
  expect((await tokens.locator('img').boundingBox())!.width).toBeGreaterThanOrEqual(160);
  unavailable = true;
  await page.reload();
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  await type.selectOption('headquarters');
  await search.fill('oa-116-001-1st-armored-div-hq');
  await page.locator('.catalog-list .catalog-card').click();
  await expect(tokens.locator('[data-state="unavailable"]')).toContainText('private-review source assets are required');
  await expect(page.locator('#catalog-detail .catalog-base-cpa > strong')).toHaveText('30*');
  await page.route('**/data/unit-reference.json', route => route.fulfill({ status: 503, body: 'Unavailable' }));
  await page.reload();
  await page.getByRole('button', { name: 'Unit catalog', exact: true }).click();
  await type.selectOption('headquarters');
  await search.fill('oa-116-001-1st-armored-div-hq');
  await page.locator('.catalog-list .catalog-card').click();
  await expect(tokens).toContainText('Original-token references could not be loaded');
  await expect(page.locator('#catalog-detail .catalog-base-cpa > strong')).toHaveText('30*');
});

test('full-board workspace removes the scenario and restores modal keyboard focus', async ({ page }) => {
  const errors = collectErrors(page);
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

test('keyboard restoration keeps roster and map controls in their originating region', async ({ page }) => {
  const errors = collectErrors(page);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.goto('/');
  await addCounter(page);
  for (const [selector, key] of [
    ['.board-roster-unit', 'Enter'],
    ['.board-roster-unit', 'Escape'],
    ['.add-counter', 'Escape'],
    ['.counter-marker', 'Enter'],
    ['.counter-marker', 'Escape'],
    ['#board-top [data-action=reinforcements]', 'Escape'],
  ]) {
    const origin = page.locator(selector);
    await origin.focus();
    await page.keyboard.press(key);
    await expect.soft(origin, `${selector} after ${key}`).toBeFocused({ timeout: 1000 });
  }
  await page.locator('.add-counter').focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog', { name: 'Reinforcements', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect.soft(page.locator('.add-counter')).toBeFocused({ timeout: 1000 });
  expect(errors).toEqual([]);
});

test('an extremely narrow viewport does not trap action positioning', async ({ page }) => {
  const errors = collectErrors(page);
  await page.goto('/');
  await addCounter(page);
  await page.locator('.order-panel > summary').click();
  await page.locator('.unit-panel > summary').click();
  await page.setViewportSize({ width: 220, height: 800 });
  const map = page.locator('.research-viewport');
  await map.focus();
  const before = await map.getAttribute('data-center-x');
  await page.keyboard.press('ArrowRight');
  await expect(map).not.toHaveAttribute('data-center-x', before!);
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.getByRole('button', { name: 'Remove planning counter' }).click();
  await expect(page.locator('.counter-marker')).toHaveCount(0);
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
  const errors = collectErrors(page);
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

test('selected actions stay reachable at bottom, sides and corners after panning', async ({ page }) => {
  const errors = collectErrors(page);
  for (const viewport of [page.viewportSize()!, { width: 390, height: 844 }]) {
    await page.setViewportSize(viewport);
    await page.goto('/');
    await page.getByRole('button', { name: 'Map detail scale', exact: true }).click();
    await addCounter(page);
    await page.locator('.order-panel > summary').click();
    await page.locator('.unit-panel > summary').click();
    const { width, height } = viewport;
    for (const [edge, x, y] of [
      ['bottom', width / 2, height - 20],
      ['left', 20, Math.floor(height * .65)],
      ['right', width - 20, Math.floor(height * .65)],
      ['bottom left', 20, height - 20],
      ['bottom right', width - 20, height - 20],
    ] as const) {
      // Re-selecting the roster row centers the actual counter, without modifying state.
      await page.locator('.order-panel > summary').click();
      await page.locator('.board-roster-unit').click();
      await page.locator('.order-panel > summary').click();
      const marker = (await page.locator('.counter-marker').boundingBox())!;
      const dx = x - marker.x - marker.width / 2;
      const dy = y - marker.y - marker.height / 2;
      const startX = width / 2 + (dx > 0 ? -90 : 90), startY = height / 2 - 120;
      await page.mouse.move(startX, startY);
      await page.mouse.down();
      await page.mouse.move(startX + dx, startY + dy, { steps: 10 });
      await page.mouse.up();
      const moved = (await page.locator('.counter-marker').boundingBox())!;
      expect(moved.x + moved.width / 2).toBeCloseTo(x, 1);
      expect(moved.y + moved.height / 2).toBeCloseTo(y, 1);
      const bar = (await page.locator('.counter-actions').boundingBox())!;
      const label = `${width}px ${edge}`;
      expect.soft(bar.x, label).toBeGreaterThanOrEqual(8);
      expect.soft(bar.x + bar.width, label).toBeLessThanOrEqual(width - 8);
      expect.soft(bar.y, label).toBeGreaterThanOrEqual(8);
      expect.soft(bar.y + bar.height, label).toBeLessThanOrEqual(height - 8);
      if (edge.startsWith('bottom')) expect.soft(bar.y + bar.height, label).toBeLessThan(moved.y);
      expect.soft(await page.locator('.counter-actions button').evaluateAll(buttons => buttons
        .filter(button => {
          const r = button.getBoundingClientRect();
          return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('button') !== button;
        }).map(button => button.getAttribute('aria-label'))), label).toEqual([]);
      await page.getByRole('button', { name: 'Reposition planning counter', exact: true }).focus();
      await page.keyboard.press('Tab');
      const tip = page.getByRole('tooltip', { name: 'Attack — requires an active simulation', exact: true });
      await expect(tip).toBeVisible();
      const tipBox = (await tip.boundingBox())!;
      expect(tipBox.x).toBeGreaterThanOrEqual(0);
      expect(tipBox.x + tipBox.width).toBeLessThanOrEqual(width);
      expect(tipBox.y).toBeGreaterThanOrEqual(0);
      expect(tipBox.y + tipBox.height).toBeLessThanOrEqual(height);
      await page.keyboard.press('Enter');
      await expect(page.getByRole('dialog')).toContainText('No active simulation');
      await page.keyboard.press('Escape');
    }
  }
  expect(errors).toEqual([]);
});

test('open floating panels do not intercept selected actions', async ({ page }) => {
  const errors = collectErrors(page);
  for (const [width, x, y] of [[1280, 950, 200], [1280, 300, 200], [1280, 1250, 650], [390, 195, 200]]) {
    await page.setViewportSize({ width, height: 800 });
    await page.goto('/');
    await page.getByRole('button', { name: 'Reinforcements', exact: true }).click();
    await page.getByRole('searchbox', { name: 'Find an identity' }).fill('2nd Bays');
    await page.getByRole('button', { name: 'Add planning counter', exact: true }).click();
    await page.mouse.click(x, y);
    await expect(page.locator('.counter-marker')).toHaveCount(1);
    if (width === 390) {
      // Put the snapped marker's center in the visible gap between the panels.
      const marker = (await page.locator('.counter-marker').boundingBox())!;
      await page.mouse.move(100, 500);
      await page.mouse.down();
      await page.mouse.move(100 + Math.round(x - marker.x - marker.width / 2), 500);
      await page.mouse.up();
    }
    for (const panel of ['.order-panel', '.unit-panel']) await expect(page.locator(panel)).toHaveAttribute('open');
    const panel = page.locator(x < width / 2 ? '.order-panel' : '.unit-panel');
    for (const open of [true, false, true]) {
      if (await panel.evaluate(p => p.hasAttribute('open')) !== open) await panel.locator('summary').click();
      await expect.poll(() => page.locator('.counter-actions button, .floating-panel > summary, .counter-marker').evaluateAll(controls => controls
        .filter(control => {
          const r = control.getBoundingClientRect();
          return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2)?.closest('button, summary') !== control;
        }).map(control => control.getAttribute('aria-label') || control.textContent)),
      { message: `${width}px placement ${x},${y}, panel open=${open}` }).toEqual([]);
      if (!open && width === 1280 && y === 200) {
        await expect.poll(async () => {
          const marker = (await page.locator('.counter-marker').boundingBox())!;
          const bar = (await page.locator('.counter-actions').boundingBox())!;
          return Math.abs(bar.x + bar.width / 2 - marker.x - marker.width / 2);
        }).toBeLessThan(1);
      }
    }
    await page.getByRole('button', { name: 'Resupply — requires an active simulation' }).click();
    await expect(page.getByRole('dialog')).toContainText('No active simulation');
    await page.keyboard.press('Escape');
    await page.getByRole('button', { name: 'Remove planning counter' }).click();
    await expect(page.locator('.counter-marker')).toHaveCount(0);
  }
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
