// A deliberately small, deterministic teaching scenario, not a rules-complete CNA engine.
export const COLS = 15;
export const ROWS = 10;
export const STAGE_LIMIT = 12;
export const TOBRUK = { c: 3, r: 3 };
export const DEPOT = { c: 13, r: 7 };
export const COAST = [
  [0, 173],
  [100, 183],
  [180, 175],
  [235, 205],
  [280, 204],
  [330, 191],
  [395, 218],
  [460, 239],
  [535, 245],
  [600, 273],
  [647, 311],
  [700, 304],
  [745, 332],
  [795, 375],
  [860, 365],
  [935, 348],
  [1000, 351],
  [1120, 318],
];
export const position = ({ c, r }) => ({
  x: 44 + c * 72 + (r % 2) * 36,
  y: 42 + r * 62.35,
});
export const hexKey = ({ c, r }) => `${c},${r}`;
export function distance(a, b) {
  const aq = a.c - (a.r - (a.r & 1)) / 2;
  const bq = b.c - (b.r - (b.r & 1)) / 2;
  return (
    (Math.abs(aq - bq) + Math.abs(a.r - b.r) + Math.abs(aq + a.r - bq - b.r)) /
    2
  );
}
export function terrain(hex) {
  const { c, r } = hex;
  if (c < 0 || c >= COLS || r < 0 || r >= ROWS) return "sea";
  const { x, y } = position(hex);
  const i = COAST.findIndex(([cx]) => cx >= x);
  const [x1, y1] = COAST[Math.max(0, i - 1)];
  const [x2, y2] = COAST[i];
  const coastline = y1 + (y2 - y1) * ((x - x1) / (x2 - x1));
  if (y < coastline + 8) return "sea";
  if ((r === 6 && c >= 2 && c <= 6) || (r === 8 && c >= 10 && c <= 12))
    return "ridge";
  return "desert";
}
export const HEXES = Array.from({ length: COLS * ROWS }, (_, i) => ({
  c: i % COLS,
  r: Math.floor(i / COLS),
})).filter((h) => terrain(h) !== "sea");
export const neighbors = (hex) => HEXES.filter((h) => distance(h, hex) === 1);
export const unitAt = (state, hex) =>
  state.units.find((u) => u.hp > 0 && u.c === hex.c && u.r === hex.r);

function makeUnit(id, name, short, side, type, c, r) {
  return {
    id,
    name,
    short,
    side,
    type,
    c,
    r,
    hp: 100,
    cp: type === "infantry" ? 3 : 5,
    maxCp: type === "infantry" ? 3 : 5,
    fuel: 100,
    water: 100,
    ammo: 100,
    entrenched: false,
  };
}
export function newGame() {
  return {
    stage: 1,
    outcome: null,
    units: [
      makeUnit("7arm", "7th Armoured Brigade", "7 AB", "allied", "tank", 8, 5),
      makeUnit("4arm", "4th Armoured Brigade", "4 AB", "allied", "tank", 9, 7),
      makeUnit(
        "22inf",
        "22nd Guards Brigade",
        "22 GD",
        "allied",
        "infantry",
        10,
        6,
      ),
      makeUnit(
        "supply",
        "Royal Army Service Corps",
        "RASC",
        "allied",
        "truck",
        10,
        8,
      ),
      makeUnit("15pz", "15th Panzer Division", "15 PZ", "axis", "tank", 6, 5),
      makeUnit("21pz", "21st Panzer Division", "21 PZ", "axis", "tank", 5, 7),
      makeUnit("garrison", "Tobruk Garrison", "GAR", "axis", "infantry", 3, 3),
    ],
    log: [
      {
        stage: 1,
        text: "Reconnaissance reports Axis armour west of the frontier.",
        type: "info",
      },
      {
        stage: 1,
        text: "Orders received. Advance west and secure Tobruk.",
        type: "order",
      },
    ],
  };
}
function log(state, text, type = "info") {
  state.log.unshift({ stage: state.stage, text, type });
  state.log = state.log.slice(0, 40);
}
function ready(state, unit, side = "allied") {
  return !state.outcome && unit && unit.hp > 0 && unit.side === side;
}

// Dijkstra keeps the highlighted range and the actual move on the same terrain-cost contract.
export function reachable(state, unit) {
  const costs = new Map();
  if (!unit || unit.hp <= 0 || state.outcome || unit.water <= 0) return costs;
  const budget = Math.min(
    unit.cp,
    unit.type === "infantry" ? Infinity : Math.floor(unit.fuel / 8),
  );
  costs.set(hexKey(unit), 0);
  const queue = [{ c: unit.c, r: unit.r, cost: 0 }];
  while (queue.length) {
    queue.sort((a, b) => a.cost - b.cost);
    const current = queue.shift();
    if (current.cost > costs.get(hexKey(current))) continue;
    // Entering an enemy zone of control ends movement; leaving it next order is allowed.
    if (
      current.cost &&
      state.units.some(
        (u) => u.hp > 0 && u.side !== unit.side && distance(u, current) === 1,
      )
    )
      continue;
    for (const next of neighbors(current)) {
      if (unitAt(state, next) && unitAt(state, next).id !== unit.id) continue;
      const cost = current.cost + (terrain(next) === "ridge" ? 2 : 1);
      const key = hexKey(next);
      if (cost <= budget && cost < (costs.get(key) ?? Infinity)) {
        costs.set(key, cost);
        queue.push({ ...next, cost });
      }
    }
  }
  costs.delete(hexKey(unit));
  return costs;
}
export function checkOutcome(state) {
  const occupant = unitAt(state, TOBRUK);
  if (occupant?.side === "allied" && occupant.type !== "truck")
    state.outcome = "victory";
  else if (
    !state.units.some(
      (u) => u.side === "allied" && u.type !== "truck" && u.hp > 0,
    ) ||
    state.stage > STAGE_LIMIT
  )
    state.outcome = "defeat";
  return state.outcome;
}
export function moveUnit(state, id, destination) {
  const unit = state.units.find((u) => u.id === id);
  if (!ready(state, unit))
    return { ok: false, message: "Select an active Commonwealth unit." };
  const cost = reachable(state, unit).get(hexKey(destination));
  if (!cost)
    return {
      ok: false,
      message: "That hex is not reachable. Choose a highlighted hex.",
    };
  unit.c = destination.c;
  unit.r = destination.r;
  unit.cp -= cost;
  if (unit.type !== "infantry") unit.fuel -= cost * 8;
  unit.entrenched = false;
  log(
    state,
    `${unit.short} advanced to ${unit.c + 1}:${unit.r + 1}. −${cost} capability point${cost === 1 ? "" : "s"}.`,
    "move",
  );
  checkOutcome(state);
  return {
    ok: true,
    message: `${unit.name} has moved. ${unit.cp} CP remaining.`,
  };
}
export function canAttack(state, attacker, target) {
  return (
    !state.outcome &&
    attacker?.hp > 0 &&
    target?.hp > 0 &&
    attacker.side !== target.side &&
    attacker.type !== "truck" &&
    attacker.cp >= 2 &&
    attacker.ammo >= 20 &&
    attacker.water > 0 &&
    distance(attacker, target) === 1
  );
}
export function combatPreview(attacker, target) {
  const cover =
    (terrain(target) === "ridge" ? 8 : 0) + (target.entrenched ? 10 : 0);
  const damage = Math.max(
    8,
    Math.round(((attacker.type === "tank" ? 40 : 27) * attacker.hp) / 100) -
      cover,
  );
  const retaliation =
    target.type === "truck" || target.ammo < 10
      ? 0
      : Math.max(
          4,
          Math.round(
            (((target.type === "tank" ? 18 : 12) * target.hp) / 100) *
              (target.water > 0 ? 1 : 0.5),
          ),
        );
  return { damage, retaliation };
}
export function attack(state, attackerId, targetId, side = "allied") {
  const a = state.units.find((u) => u.id === attackerId);
  const b = state.units.find((u) => u.id === targetId);
  if (!ready(state, a, side) || !canAttack(state, a, b))
    return {
      ok: false,
      message:
        "Assault needs an adjacent enemy, 2 CP, water, and 20 ammunition.",
    };
  const { damage, retaliation } = combatPreview(a, b);
  a.cp -= 2;
  a.ammo -= 20;
  a.entrenched = false;
  b.hp = Math.max(0, b.hp - damage);
  if (b.hp > 0) {
    a.hp = Math.max(0, a.hp - retaliation);
    if (retaliation) b.ammo = Math.max(0, b.ammo - 10);
  }
  const message = `${a.short} assaulted ${b.short}: ${damage} damage${b.hp <= 0 ? ". Enemy formation destroyed." : `, ${retaliation} returned.`}`;
  log(state, message, "combat");
  checkOutcome(state);
  return { ok: true, message };
}
export function digIn(state, id) {
  const unit = state.units.find((u) => u.id === id);
  if (
    !ready(state, unit) ||
    unit.cp < 1 ||
    unit.entrenched ||
    unit.type === "truck"
  )
    return {
      ok: false,
      message:
        "Digging in needs 1 CP and a combat unit not already entrenched.",
    };
  unit.entrenched = true;
  unit.cp -= 1;
  log(state, `${unit.short} dug in. Incoming damage reduced by 10.`, "order");
  return {
    ok: true,
    message: `${unit.name} is dug in. Moving or attacking removes cover.`,
  };
}
export function supplySource(state, unit) {
  if (!unit) return null;
  if (distance(unit, DEPOT) <= 2) return "depot";
  return (
    state.units.find(
      (u) =>
        u.side === "allied" &&
        u.type === "truck" &&
        u.hp > 0 &&
        u.id !== unit.id &&
        u.water >= 25 &&
        u.fuel >= 25 &&
        u.ammo >= 25 &&
        distance(u, unit) <= 2,
    ) ?? null
  );
}
export function resupply(state, id) {
  const unit = state.units.find((u) => u.id === id);
  const source = supplySource(state, unit);
  if (!ready(state, unit) || unit.cp < 1 || !source)
    return {
      ok: false,
      message:
        "Resupply needs 1 CP and a stocked truck or depot within 2 hexes.",
    };
  if (unit.fuel === 100 && unit.water === 100 && unit.ammo === 100)
    return { ok: false, message: "This formation is already fully supplied." };
  const before = [unit.fuel, unit.water, unit.ammo];
  for (const resource of ["fuel", "water", "ammo"]) {
    const amount = Math.min(
      100 - unit[resource],
      source === "depot" ? 100 : 25,
    );
    unit[resource] += amount;
    if (source !== "depot") source[resource] -= amount;
  }
  unit.cp -= 1;
  log(
    state,
    `${unit.short} resupplied from ${source === "depot" ? "the frontier depot" : "RASC trucks"}.`,
    "supply",
  );
  return {
    ok: true,
    message: `Supplies received: +${unit.fuel - before[0]} fuel, +${unit.water - before[1]} water, +${unit.ammo - before[2]} ammunition.`,
  };
}
export function endStage(state) {
  if (state.outcome)
    return {
      ok: false,
      message: "This operation has ended. Start a new operation to play again.",
    };
  for (const enemy of state.units.filter(
    (u) => u.side === "axis" && u.hp > 0,
  )) {
    enemy.cp = enemy.maxCp;
    const targets = state.units
      .filter((u) => u.side === "allied" && u.hp > 0)
      .sort((a, b) => distance(enemy, a) - distance(enemy, b));
    const target = targets[0];
    if (!target) break;
    if (enemy.id !== "garrison" && distance(enemy, target) > 1) {
      const moves = reachable(state, enemy);
      const best = HEXES.filter(
        (h) => moves.has(hexKey(h)) && moves.get(hexKey(h)) <= 2,
      ).sort((a, b) => distance(a, target) - distance(b, target))[0];
      if (best && distance(best, target) < distance(enemy, target)) {
        const cost = moves.get(hexKey(best));
        enemy.c = best.c;
        enemy.r = best.r;
        enemy.cp -= cost;
        enemy.fuel -= cost * 8;
        enemy.entrenched = false;
        log(
          state,
          `${enemy.short} repositioned toward Commonwealth forces.`,
          "move",
        );
      }
    }
    if (canAttack(state, enemy, target))
      attack(state, enemy.id, target.id, "axis");
    if (state.outcome)
      return { ok: true, message: "The last combat formation has been lost." };
  }
  state.stage += 1;
  const hot = state.stage % 3 === 0;
  for (const unit of state.units.filter((u) => u.hp > 0)) {
    unit.water = Math.max(0, unit.water - (hot ? 18 : 10));
    if (unit.type !== "infantry")
      unit.fuel = Math.max(0, unit.fuel - (hot ? 5 : 2));
    if (!unit.water) unit.hp = Math.max(0, unit.hp - 10);
    unit.cp = unit.maxCp;
  }
  log(
    state,
    hot
      ? "Hot winds. Water consumption increased to 18; vehicle fuel loss to 5."
      : "Supply upkeep: −10 water and −2 vehicle fuel per formation.",
    "supply",
  );
  checkOutcome(state);
  return {
    ok: true,
    message: state.outcome
      ? "The operation has ended."
      : `Operations stage ${state.stage}. Commonwealth orders are open.`,
  };
}
export function dateLabel(stage) {
  const date = new Date(
    Date.UTC(1941, 10, 18 + Math.floor(((stage - 1) * 7) / 3)),
  );
  return date.toLocaleDateString("en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  });
}
