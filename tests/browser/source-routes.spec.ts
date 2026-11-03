import { expect, test } from '@playwright/test';

test('source road and rail bends stay registered through pan, zoom and layer toggles', async ({ page }) => {
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'No private scan' }));
  await page.goto('/');
  // Independent source-pixel samples: road fill and rail spine, not hex centers
  // or coordinates read back from the generated curve file.
  const samples = [
    { hex: 'D3418', kind: 'road', x: 10000, y: 2029.5 },
    { hex: 'D3418', kind: 'railroad', x: 10000, y: 2040.5 },
    { hex: 'E3324', kind: 'railroad', x: 13320, y: 1938.5 },
  ];
  for (const sample of samples) {
    await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill(sample.hex);
    await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
    await page.locator('.geography-details > summary').click();
    const paths = page.locator(`.source-route.${sample.kind}`);
    async function checkPosition() {
      await expect(paths.first()).toBeVisible();
      const error = await paths.evaluateAll((nodes, point) => {
        const viewport = document.querySelector<HTMLElement>('.research-viewport')!;
        const bounds = viewport.getBoundingClientRect();
        const zoom = Number(viewport.dataset.zoom);
        const expected = new DOMPoint(
          bounds.x + bounds.width / 2 + (point.x - Number(viewport.dataset.centerX)) * zoom,
          bounds.y + bounds.height / 2 + (point.y - Number(viewport.dataset.centerY)) * zoom,
        );
        let nearest = Infinity;
        for (const node of nodes as SVGPathElement[]) {
          const box = node.getBBox();
          if (point.x < box.x - 3 || point.x > box.x + box.width + 3 ||
              point.y < box.y - 3 || point.y > box.y + box.height + 3) continue;
          const length = node.getTotalLength();
          const matrix = node.getScreenCTM()!;
          for (let distance = 0; distance <= length; distance += 1) {
            const p = node.getPointAtLength(distance).matrixTransform(matrix);
            nearest = Math.min(nearest, Math.hypot(p.x - expected.x, p.y - expected.y) / zoom);
          }
        }
        return nearest;
      }, sample);
      expect(error).toBeLessThan(2);
    }
    await checkPosition();
    await page.locator('.research-viewport').focus();
    await page.keyboard.press('ArrowRight');
    await page.keyboard.press('+');
    await checkPosition();
    await page.getByRole('button', { name: 'Original terrain', exact: true }).click();
    await checkPosition();
    await page.getByRole('button', { name: 'Original terrain', exact: true }).click();
    await page.getByRole('button', { name: 'Geography', exact: true }).click();
    await expect(page.locator('.source-route')).toHaveCount(0);
    await page.getByRole('button', { name: 'Geography', exact: true }).click();
    await checkPosition();
  }
  await expect(page.locator('.source-route-layer > title')).toContainText('not movement authority');
  await expect(page.locator('.geography-link.road:not(.source-route), .geography-link.railroad:not(.source-route)')).toHaveCount(0);
});

test('untraced crossings remain visible evidence without fabricated route chords', async ({ page }) => {
  await page.route('**/tiles/map/manifest.json', route => route.fulfill({ status: 404, body: 'No private scan' }));
  await page.goto('/');
  await page.getByRole('textbox', { name: 'Hex identity', exact: true }).fill('A2109');
  await page.getByRole('button', { name: 'Inspect hex', exact: true }).click();
  await expect(page.locator('.geography-inspector')).toContainText('Source curve incomplete here');
  const markers = page.locator('.geography-overlay .geography-route-untraced[data-hex="A2109"]');
  expect(await markers.count()).toBeGreaterThan(0);
  await expect(markers.first().locator('title')).toContainText('side marker is not the route');
  await expect(markers.first()).toHaveCSS('stroke-dasharray', '2px, 4px');
  await page.getByRole('button', { name: 'Geography', exact: true }).click();
  await expect(markers).toHaveCount(0);
  await page.getByRole('button', { name: 'Geography', exact: true }).click();
  expect(await markers.count()).toBeGreaterThan(0);
  await page.locator('.geography-legend > summary').click();
  await expect(page.getByRole('region', { name: 'Map legend', exact: true })).toContainText('Amber dotted sides retain observed road/rail crossings');
});
