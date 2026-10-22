import test from "node:test";
import assert from "node:assert/strict";
import {
  newGame,
  moveUnit,
  reachable,
  terrain,
  attack,
  digIn,
  resupply,
  endStage,
  checkOutcome,
  TOBRUK,
  STAGE_LIMIT,
  dateLabel,
} from "./game.js";

test("scenario starts with distinct, passable positions and independent state", () => {
  const state = newGame();
  assert.equal(new Set(state.units.map((u) => `${u.c},${u.r}`)).size, 7);
  for (const unit of state.units) assert.notEqual(terrain(unit), "sea");
  state.units[0].hp = 0;
  assert.equal(newGame().units[0].hp, 100);
});

test("movement charges terrain and fuel, rejects sea and occupied hexes without mutations", () => {
  const state = newGame();
  const tank = state.units[0];
  Object.assign(tank, { c: 8, r: 6 });
  assert.equal(moveUnit(state, tank.id, { c: 7, r: 6 }).ok, true);
  assert.deepEqual([tank.c, tank.r, tank.cp, tank.fuel], [7, 6, 4, 92]);
  // The next hex west is a ridge and costs twice as much as desert.
  assert.equal(moveUnit(state, tank.id, { c: 6, r: 6 }).ok, true);
  assert.deepEqual([tank.c, tank.r, tank.cp, tank.fuel], [6, 6, 2, 76]);
  const before = structuredClone(state);
  assert.equal(moveUnit(state, tank.id, { c: 6, r: 5 }).ok, false);
  assert.equal(moveUnit(state, tank.id, { c: 8, r: 0 }).ok, false);
  assert.deepEqual(state, before);
});

test("fuel and water constrain travel; infantry does not spend fuel", () => {
  const state = newGame();
  const tank = state.units[0];
  tank.fuel = 7;
  assert.equal(reachable(state, tank).size, 0);
  tank.fuel = 8;
  assert.equal(moveUnit(state, tank.id, { c: 7, r: 5 }).ok, true);
  assert.equal(tank.fuel, 0);
  tank.water = 0;
  assert.equal(reachable(state, tank).size, 0);
  const infantry = state.units[2];
  infantry.fuel = 0;
  assert.equal(moveUnit(state, infantry.id, { c: 11, r: 6 }).ok, true);
  assert.equal(infantry.fuel, 0);
});

test("an enemy zone stops a path, and players cannot move Axis formations", () => {
  const state = newGame();
  const tank = state.units[0];
  tank.cp = 2;
  assert.equal(reachable(state, tank).get("7,5"), 1);
  assert.equal(reachable(state, tank).has("7,4"), false);
  const withoutEnemies = structuredClone(state);
  withoutEnemies.units = withoutEnemies.units.filter(
    (u) => u.side === "allied",
  );
  assert.equal(reachable(withoutEnemies, tank).get("7,4"), 2);
  const before = structuredClone(state);
  assert.equal(moveUnit(state, "15pz", { c: 7, r: 5 }).ok, false);
  assert.deepEqual(state, before);
});

test("assault spends CP and ammunition, returns fire only from surviving defenders", () => {
  const state = newGame();
  const a = state.units[0];
  const b = state.units.find((u) => u.id === "15pz");
  assert.equal(attack(state, a.id, b.id).ok, false);
  moveUnit(state, a.id, { c: 7, r: 5 });
  a.ammo = 19;
  const undersupplied = structuredClone(state);
  assert.equal(attack(state, a.id, b.id).ok, false);
  assert.deepEqual(state, undersupplied);
  a.ammo = 100;
  assert.equal(attack(state, a.id, b.id).ok, true);
  assert.deepEqual([a.hp, b.hp, a.cp, a.ammo, b.ammo], [82, 60, 2, 80, 90]);
  b.hp = 8;
  assert.equal(attack(state, a.id, b.id).ok, true);
  assert.deepEqual([a.hp, b.hp, a.cp, a.ammo], [82, 0, 0, 60]);
  const before = structuredClone(state);
  assert.equal(attack(state, a.id, b.id).ok, false);
  assert.deepEqual(state, before);
});

test("digging in reduces damage, is not stackable, and moving removes protection", () => {
  const state = newGame();
  const a = state.units[0];
  const b = state.units.find((u) => u.id === "15pz");
  moveUnit(state, a.id, { c: 7, r: 5 });
  assert.equal(digIn(state, a.id).ok, true);
  assert.equal(digIn(state, a.id).ok, false);
  assert.equal(attack(state, b.id, a.id, "axis").ok, true);
  assert.equal(a.hp, 70);
  assert.equal(moveUnit(state, a.id, { c: 8, r: 5 }).ok, true);
  assert.equal(a.entrenched, false);
});

test("truck resupply transfers actual shortages, respects range, and depot refills trucks", () => {
  const state = newGame();
  const tank = state.units.find((u) => u.id === "4arm");
  const truck = state.units.find((u) => u.id === "supply");
  Object.assign(tank, { fuel: 80, water: 40, ammo: 95 });
  assert.equal(resupply(state, tank.id).ok, true);
  assert.deepEqual(
    [tank.fuel, tank.water, tank.ammo, tank.cp],
    [100, 65, 100, 4],
  );
  assert.deepEqual([truck.fuel, truck.water, truck.ammo], [80, 75, 95]);
  state.units[0].fuel = 80;
  const outOfRange = structuredClone(state);
  assert.equal(resupply(state, "7arm").ok, false);
  assert.deepEqual(state, outOfRange);
  Object.assign(truck, { c: 12, r: 7, fuel: 8, water: 10, ammo: 20 });
  assert.equal(resupply(state, truck.id).ok, true);
  assert.deepEqual(
    [truck.fuel, truck.water, truck.ammo, truck.cp],
    [100, 100, 100, 4],
  );
});

test("end stage executes enemy actions, upkeep, hot weather, and restores CP", () => {
  const state = newGame();
  const tank = state.units[0];
  digIn(state, tank.id);
  endStage(state);
  assert.equal(state.stage, 2);
  assert.ok(tank.hp < 100, "Axis should close and attack");
  assert.equal(tank.cp, 5);
  assert.deepEqual([tank.water, tank.fuel], [90, 98]);
  endStage(state);
  assert.equal(state.stage, 3);
  assert.deepEqual([tank.water, tank.fuel], [72, 93]);
  assert.equal(dateLabel(4), "25 Nov 1941");
});

test("dehydration prevents assault and causes losses at the next upkeep", () => {
  const state = newGame();
  const tank = state.units[0];
  moveUnit(state, tank.id, { c: 7, r: 5 });
  tank.water = 0;
  assert.equal(attack(state, tank.id, "15pz").ok, false);
  state.units = state.units.filter((u) => u.side === "allied");
  endStage(state);
  assert.equal(tank.hp, 90);
});

test("capture wins, trucks cannot capture, losses and deadline end play", () => {
  const state = newGame();
  state.units.find((u) => u.id === "garrison").hp = 0;
  Object.assign(state.units[0], { c: 4, r: 3 });
  assert.equal(moveUnit(state, "7arm", TOBRUK).ok, true);
  assert.equal(state.outcome, "victory");
  const before = structuredClone(state);
  assert.equal(endStage(state).ok, false);
  assert.equal(digIn(state, "7arm").ok, false);
  assert.deepEqual(state, before);

  const truckOnly = newGame();
  truckOnly.units.find((u) => u.id === "garrison").hp = 0;
  Object.assign(
    truckOnly.units.find((u) => u.id === "supply"),
    TOBRUK,
  );
  assert.equal(checkOutcome(truckOnly), null);
  for (const unit of truckOnly.units.filter(
    (u) => u.side === "allied" && u.type !== "truck",
  ))
    unit.hp = 0;
  assert.equal(checkOutcome(truckOnly), "defeat");

  const deadline = newGame();
  deadline.units = deadline.units.filter((u) => u.side === "allied");
  deadline.stage = STAGE_LIMIT;
  assert.equal(checkOutcome(deadline), null);
  endStage(deadline);
  assert.equal(deadline.outcome, "defeat");
});
