import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

test('every source curve retains typed model evidence and a continuous measured chain', () => {
  const routes = JSON.parse(readFileSync('data/map/source-paths.json', 'utf8'));
  const model = JSON.parse(execFileSync('cargo', ['run', '--quiet', '--locked', '-p', 'cna-map', '--bin', 'map-model'], { maxBuffer: 64 * 1024 * 1024 }));
  assert.equal(routes.source_sha256, model.sources[routes.source_ref]);
  const key = (kind, survey, side) => `${kind}/${survey}/${side.hex}/${side.direction}`;
  const observed = new Set(model.surveys.flatMap(survey => survey.records
    .filter(r => r.kind === 'hexside' && ['reviewed', 'provisional'].includes(r.record.observation.status))
    .map(r => key(r.record.observation.value, survey.id, r.side))));
  assert.ok(routes.paths.some(p => p.depiction === 'road'));
  assert.ok(routes.paths.some(p => p.depiction === 'railroad'));
  for (const path of routes.paths) {
    assert.ok(['road', 'railroad'].includes(path.depiction));
    assert.equal(path.review_status, 'provisional');
    assert.ok(path.observation_refs.length > 0);
    for (const ref of path.observation_refs) assert.ok(observed.has(key(path.depiction, ref.survey_id, ref.side)), path.id);
    assert.deepEqual(path.quadratics[0][0], path.raw_points[0]);
    assert.deepEqual(path.quadratics.at(-1)[2], path.raw_points.at(-1));
    path.quadratics.slice(1).forEach((q, i) => assert.deepEqual(q[0], path.quadratics[i][2]));
    path.raw_points.slice(1).forEach(([x, y], i) => {
      const [a, b] = path.raw_points[i];
      assert.ok(Math.abs(x - a) <= 1 && Math.abs(y - b) <= 1, `${path.id}: unsupported gap`);
    });
  }
});
