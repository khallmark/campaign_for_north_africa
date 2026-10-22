import "./style.css";
import {
  newGame,
  moveUnit,
  attack,
  digIn,
  resupply,
  endStage,
  dateLabel,
  canAttack,
  combatPreview,
  supplySource,
  STAGE_LIMIT,
} from "./game.js";
import { renderMap, sprite } from "./map.js";

let game = newGame();
let selected = "7arm";
let mode = "inspect";
let grid = true;
let supplies = false;
let zoom = 1;
let modal = null;
let modalReturnFocus = null;
let targetId = null;
let message =
  "Select a formation, then issue an order. Your first objective is Tobruk.";
let messageError = false;
const app = document.querySelector("#app");
const paths = {
  compass:
    '<path d="m12 2 3 7 7 3-7 3-3 7-3-7-7-3 7-3Z"/><circle cx="12" cy="12" r="2"/>',
  arrow: '<path d="M4 12h16m-6-6 6 6-6 6"/>',
  move: '<path d="M12 3v18M3 12h18M9 6l3-3 3 3M9 18l3 3 3-3M6 9l-3 3 3 3M18 9l3 3-3 3"/>',
  attack:
    '<circle cx="12" cy="12" r="7"/><path d="M12 2v5m0 10v5M2 12h5m10 0h5"/><circle cx="12" cy="12" r="2"/>',
  shield:
    '<path d="m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z"/><path d="m8 12 3 3 5-6"/>',
  box: '<path d="m12 3 9 5v9l-9 5-9-5V8Zm-9 5 9 5 9-5M12 13v9M7 5l9 5"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 1v3m0 16v3M1 12h3m16 0h3M4 4l2 2m12 12 2 2M4 20l2-2M18 6l2-2"/>',
  book: '<path d="M12 5C8 2 3 3 3 3v16s5-1 9 2c4-3 9-2 9-2V3s-5-1-9 2Zm0 0v16"/>',
  reset: '<path d="M3 10a9 9 0 1 1 1 8M3 3v7h7"/>',
  grid: '<path d="m12 2 9 5v10l-9 5-9-5V7Z"/><path d="m3 7 9 5 9-5M12 12v10"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  minus: '<path d="M5 12h14"/>',
  water:
    '<path d="M12 2S5 10 5 15a7 7 0 0 0 14 0c0-5-7-13-7-13Z"/><path d="M8 15a4 4 0 0 0 4 4"/>',
  fuel: '<path d="M5 3h10v18H5Zm0 7h10M3 21h14M15 7h3l3 3v8c0 2-3 2-3 0v-5h-3"/>',
  ammo: '<path d="M5 21V7l3-5 3 5v14ZM15 21V7l3-5 3 5v14ZM5 15h6m4 0h6"/>',
  close: '<path d="m6 6 12 12M6 18 18 6"/>',
  flag: '<path d="M5 22V3m0 0c5-4 9 4 15 0v10c-6 4-10-4-15 0"/>',
};
const icon = (name) =>
  `<svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${paths[name] || paths.compass}</svg>`;
const sideLabel = (u) => (u.side === "allied" ? "Commonwealth" : "Axis forces");
const typeLabel = (u) =>
  u.type === "tank"
    ? "Armoured formation"
    : u.type === "infantry"
      ? "Infantry formation"
      : "Supply & transport";

function actionButton(action, label, symbol, disabled = false, extra = "") {
  return `<button data-action="${action}" aria-label="${label}" ${disabled ? "disabled" : ""} class="${extra}">${icon(symbol)}<span>${label}</span></button>`;
}

function renderPanel(unit) {
  if (!unit)
    return '<div class="empty-panel"><h2>No formation selected</h2><p>Select a surviving formation on the map or in the order of battle.</p></div>';
  const friendly = unit.side === "allied";
  const source = supplySource(game, unit);
  const active = friendly && !game.outcome;
  const enemies = game.units.filter((u) => canAttack(game, unit, u));
  return `<div class="panel-heading"><span>Formation details</span><span class="hex-coordinate">${String(unit.c + 1).padStart(2, "0")}:${String(unit.r + 1).padStart(2, "0")}</span></div>
    <section class="unit-detail">
      <div class="allegiance"><i class="faction-dot ${unit.side}"></i>${sideLabel(unit)}<span>${unit.type === "tank" ? "▱" : unit.type === "infantry" ? "×" : "◇"}</span></div>
      <div class="unit-portrait"><div class="portrait-ring"></div><img src="${sprite(unit)}" alt="${unit.type === "tank" ? (friendly ? "Crusader tank" : "Panzer III tank") : typeLabel(unit)}"/><span class="unit-stamp">${unit.short}</span></div>
      <h2>${unit.name}</h2><p class="unit-subtitle">${typeLabel(unit)}</p>
      <div class="strength-line"><span>Combat strength</span><strong>${unit.hp}<small>%</small></strong></div>
      <div class="strength-track"><span style="width:${unit.hp}%"></span></div>
      <div class="capability"><span>Capability points</span><div aria-label="${unit.cp} of ${unit.maxCp} capability points">${Array.from({ length: unit.maxCp }, (_, i) => `<i class="${i < unit.cp ? "available" : ""}"></i>`).join("")}<strong>${unit.cp}/${unit.maxCp}</strong></div></div>
      ${unit.entrenched ? '<div class="dug-in">Dug in · 10 damage protection</div>' : ""}
    </section>
    <section class="supplies-panel"><div class="section-title"><h3>Field supplies</h3><span class="supply-state ${source ? "" : "isolated"}">${!friendly ? "Enemy stores" : source ? "In supply range" : "Out of supply range"}</span></div>
      ${[
        ["fuel", "Fuel"],
        ["water", "Water"],
        ["ammo", "Ammunition"],
      ]
        .map(
          ([key, label]) =>
            `<div class="resource-row ${unit[key] < 25 ? "low" : ""}">${icon(key)}<span>${label}</span><div class="resource-track"><i style="width:${unit[key]}%"></i></div><strong>${unit.type === "infantry" && key === "fuel" ? "—" : unit[key] + "%"}</strong></div>`,
        )
        .join("")}
      ${friendly ? `<p class="supply-note">${unit.type === "truck" ? "Stocked trucks supply formations within 2 hexes. Refill at the frontier depot." : "Keep your supply trucks close. The desert takes its share every stage."}</p>` : ""}
    </section>
    <section class="orders-panel"><h3>${friendly ? "Issue orders" : "Reconnaissance"}</h3>
      ${
        friendly
          ? `<div class="order-grid">
        ${actionButton("move", "Move", "move", !active || unit.cp < 1 || unit.water <= 0 || (unit.type !== "infantry" && unit.fuel < 8), mode === "move" ? "active" : "")}
        ${actionButton("attack", "Assault", "attack", !active || !enemies.length, mode === "attack" ? "active danger" : "")}
        ${actionButton("digin", unit.entrenched ? "Dug in" : "Dig in", "shield", !active || unit.cp < 1 || unit.entrenched || unit.type === "truck")}
        ${actionButton("resupply", "Resupply", "box", !active || unit.cp < 1 || !source || (unit.fuel === 100 && unit.water === 100 && unit.ammo === 100))}
      </div><p class="order-hint">${mode === "move" ? "Click a highlighted hex to move. Numbers show CP cost." : mode === "attack" ? "Choose a marked enemy to preview an assault." : "Movement costs 1–2 CP per hex. Assaults cost 2 CP."}</p>`
          : '<p class="order-hint">Enemy formations act when you end the stage. Select a Commonwealth formation to issue orders.</p>'
      }
    </section>`;
}

function renderModal() {
  if (!modal) return "";
  let body = "";
  if (modal === "help")
    body = `<span class="eyebrow">Field manual / prototype 0.1</span><h2>A war of distance.<br>A battle for supplies.</h2><p>Lead the Commonwealth west to Tobruk. This is a small, fictional training scenario inspired by the land and logistics rules of <em>The Campaign for North Africa</em>, not a historical order of battle.</p>
    <div class="manual-grid"><section>${icon("move")}<h3>Move with purpose</h3><p>Select a green formation, then Move. Desert costs 1 capability point (CP); ridges cost 2. Vehicles spend 8 fuel per CP. Entering an enemy’s adjacent hex ends that move.</p></section><section>${icon("attack")}<h3>Fight at close range</h3><p>Assault an adjacent enemy for 2 CP and 20 ammunition. Preview exact damage before committing. Dig in for 1 CP to reduce incoming damage by 10; moving or assaulting removes cover.</p></section><section>${icon("box")}<h3>Mind the supply line</h3><p>Resupply for 1 CP within 2 hexes of a stocked truck or the depot. Trucks transfer up to 25 of each resource; the depot refills stores. Zero water prevents movement and assaults.</p></section><section>${icon("sun")}<h3>The desert takes a toll</h3><p>End stage activates the Axis, consumes 10 water and 2 vehicle fuel, then restores CP. Every third stage costs 18 water and 5 fuel instead. Zero water causes 10 strength loss.</p></section></div>
    <div class="manual-objective">${icon("flag")}<p><strong>Secure Tobruk within ${STAGE_LIMIT} operations stages.</strong><br>Move a combat formation into its hex after defeating the garrison. Lose if time runs out or all combat formations are destroyed.</p></div>
    <p class="fine-print">Inspired by rules §§5, 6, 8, 15, 49 and 52. Three stages represent a week. Map, forces, costs, weather, and combat are simplified; air, naval, detailed logistics, and the pasta rule are not implemented. Newly AI-generated unit art. Session resets on page reload.</p>${actionButton("close", "Return to the map", "arrow", false, "primary")}`;
  if (modal === "reset")
    body = `<span class="eyebrow">A fresh set of orders</span><h2>Start a new operation?</h2><p>This will reset all positions, supplies, and the field journal. The current session will be lost.</p><div class="dialog-actions">${actionButton("close", "Keep playing", "close")}${actionButton("confirm-reset", "Start new operation", "reset", false, "primary")}</div>`;
  if (modal === "combat") {
    const a = game.units.find((u) => u.id === selected);
    const b = game.units.find((u) => u.id === targetId);
    const { damage, retaliation } = combatPreview(a, b);
    body = `<span class="eyebrow">Close assault / ${a.short} → ${b.short}</span><h2>Commit to the assault?</h2><p>${a.name} will engage ${b.name}.</p><div class="combat-comparison"><div><img src="${sprite(a)}" alt="Attacking formation"/><span>Commonwealth</span><strong>${a.hp}% strength</strong><small>${b.hp <= damage ? "No" : retaliation + "%"} return damage</small></div><span class="versus">vs.</span><div><img src="${sprite(b)}" alt="Defending formation"/><span>Axis forces</span><strong>${b.hp}% strength</strong><small>−${damage}% on assault</small></div></div><p class="combat-cost">Cost: <strong>2 CP</strong> and <strong>20 ammunition</strong>. ${b.hp <= damage ? "This assault will destroy the enemy formation." : "The defender will return fire if it survives."}</p><div class="dialog-actions">${actionButton("close", "Cancel", "close")}${actionButton("confirm-attack", "Launch assault", "attack", false, "primary")}</div>`;
  }
  if (modal === "outcome")
    body = `<span class="eyebrow">Operation report</span><div class="outcome-symbol">${icon("flag")}</div><h2>${game.outcome === "victory" ? "Tobruk is secured." : "The advance has ended."}</h2><p>${game.outcome === "victory" ? "Your formations have taken the objective. The road to Tobruk is open." : game.stage > STAGE_LIMIT ? "The operational window has closed. Tobruk remains beyond your lines." : "Your last combat formation has been lost. The operation cannot continue."}</p><p>Stages played: ${Math.min(game.stage, STAGE_LIMIT)} · Surviving formations: ${game.units.filter((u) => u.side === "allied" && u.hp > 0).length}/4</p><div class="dialog-actions">${actionButton("close", "Review the map", "grid")}${actionButton("confirm-reset", "Try a new operation", "reset", false, "primary")}</div>`;
  return `<dialog class="dialog ${modal === "help" ? "manual" : ""}" aria-label="${modal === "help" ? "Field manual" : modal === "combat" ? "Assault preview" : modal === "outcome" ? "Operation result" : "New operation"}"><button class="dialog-close icon-button" data-action="close" aria-label="Close dialog">${icon("close")}</button>${body}</dialog>`;
}

function render() {
  const focused = document.activeElement;
  const focusSelector = focused?.dataset.action
    ? `[data-action="${focused.dataset.action}"]`
    : focused?.dataset.unit
      ? `[data-unit="${focused.dataset.unit}"]`
      : null;
  if (modal && !app.querySelector("dialog")) modalReturnFocus = focusSelector;
  const unit = game.units.find((u) => u.id === selected && u.hp > 0);
  const friendlies = game.units.filter((u) => u.side === "allied");
  const hot = game.stage % 3 === 0;
  app.innerHTML = `
    <header class="site-header"><a href="/" class="brand" aria-label="The Campaign for North Africa home"><span class="brand-mark">${icon("compass")}</span><div><span class="brand-prefix">The Campaign for</span><span class="brand-name">North Africa</span></div></a><div class="edition"><span class="edition-line"></span><span>A desert warfare simulation<small>1940 — 1943</small></span></div><nav aria-label="Game controls">${actionButton("help", "Field manual", "book")}${actionButton("reset", "New operation", "reset")}</nav></header>
    <main>
      <section class="operation-bar"><div class="operation-heading"><div class="scenario-meta"><span class="live-dot"></span>Sandbox scenario<span class="separator">/</span><span>Western Desert</span></div><h1>The road to Tobruk</h1></div><div class="turn-controls"><div class="weather">${icon("sun")}<div>${hot ? "Hot winds" : "Clear skies"}<small>${hot ? "Increased supply loss" : "Normal supply loss"}</small></div></div><div class="turn-date"><strong>${dateLabel(game.stage)}</strong><span>Week ${Math.ceil(game.stage / 3)} <i>·</i> Stage ${((game.stage - 1) % 3) + 1} of 3</span></div>${actionButton(game.outcome ? "outcome" : "end", game.outcome ? "View result" : "End stage", "arrow", false, "primary end-stage")}</div></section>
      <div class="game-layout"><section class="map-panel" aria-label="Battlefield"><div class="map-toolbar"><div class="map-title">${icon("compass")}<span>Operational map</span><span class="map-tag">Tobruk sector</span></div><div class="map-toggles"><button data-action="grid" aria-label="Toggle hex grid" aria-pressed="${grid}" class="icon-button ${grid ? "toggled" : ""}" title="Hex grid">${icon("grid")}</button><button data-action="supplies" aria-label="Toggle supply range" aria-pressed="${supplies}" class="icon-button ${supplies ? "toggled" : ""}" title="Supply range">${icon("box")}</button></div></div>
      <div class="map-viewport">${renderMap(game, selected, { mode, grid, supplies, zoom })}<div class="objective-note">${icon("flag")}<div><strong>Secure Tobruk</strong><span>${Math.max(0, STAGE_LIMIT - game.stage + 1)} stages remaining</span></div></div><div class="map-zoom"><button data-action="zoom-in" aria-label="Zoom in" ${zoom >= 1.75 ? "disabled" : ""}>${icon("plus")}</button><button data-action="zoom-reset" aria-label="Reset map zoom">${Math.round(zoom * 100)}%</button><button data-action="zoom-out" aria-label="Zoom out" ${zoom <= 1 ? "disabled" : ""}>${icon("minus")}</button></div>${mode !== "inspect" ? `<div class="map-mode">${icon(mode === "move" ? "move" : "attack")}<span>${mode === "move" ? "Choose a highlighted destination" : "Choose an adjacent enemy"}</span><button data-action="cancel" aria-label="Cancel order">${icon("close")}</button></div>` : ""}</div>
      <div class="map-legend"><span><i class="faction-dot allied"></i>Commonwealth</span><span><i class="faction-dot axis"></i>Axis</span><span class="legend-ridge">△<span>Ridge · 2 CP</span></span><span class="map-scale-caption">Schematic map · ~8 km / hex</span></div>
      <div class="orders-status ${messageError ? "error" : ""}" role="status" aria-live="polite">${icon(messageError ? "flag" : "compass")}<span>${message}</span></div>
      </section><aside class="detail-panel" aria-label="Selected formation">${renderPanel(unit)}</aside></div>
      <section class="bottom-layout"><div class="roster-section"><div class="section-title"><h2>Order of battle <span>${friendlies.filter((u) => u.hp > 0).length} formations</span></h2><span class="your-command"><i class="faction-dot allied"></i>Your command</span></div><div class="roster">${friendlies.map((u) => `<button class="roster-unit ${u.id === selected ? "chosen" : ""} ${u.hp <= 0 ? "destroyed" : ""}" data-unit="${u.id}" aria-label="Select ${u.name}" aria-pressed="${u.id === selected}" ${u.hp <= 0 ? "disabled" : ""}><img src="${sprite(u)}" alt=""/><div><strong>${u.short === "RASC" ? "Supply column" : u.name.replace(" Brigade", "")}</strong><span>${u.hp <= 0 ? "Formation lost" : `${u.hp}% strength <i>·</i> ${u.cp} CP`}</span></div><span class="roster-dot ${u.cp === 0 ? "spent" : ""}"></span></button>`).join("")}</div></div><section class="journal"><div class="section-title"><h2>Field journal</h2><span>Latest dispatches</span></div><div class="journal-entries">${game.log
        .slice(0, 3)
        .map(
          (entry) =>
            `<div class="journal-entry ${entry.type}"><span class="journal-tick"></span><p>${entry.text}</p><time>S${entry.stage}</time></div>`,
        )
        .join("")}</div></section></section>
    </main><footer><span>Distance is the enemy. Supply is the lifeline.</span><span>Playable prototype <i>·</i> Simplified rules <i>·</i> Session only</span></footer>${renderModal()}`;
  if (modal) {
    const dialog = app.querySelector("dialog");
    dialog.showModal();
    dialog.addEventListener("cancel", (event) => {
      event.preventDefault();
      modal = null;
      render();
    });
    dialog.addEventListener("click", (event) => {
      if (event.target === dialog) {
        const box = dialog.getBoundingClientRect();
        if (
          event.clientX < box.left ||
          event.clientX > box.right ||
          event.clientY < box.top ||
          event.clientY > box.bottom
        ) {
          modal = null;
          render();
        }
      }
    });
  } else {
    const restore = modalReturnFocus || focusSelector;
    modalReturnFocus = null;
    if (restore) app.querySelector(restore)?.focus({ preventScroll: true });
  }
}

function apply(result) {
  message = result.message;
  messageError = !result.ok;
  if (game.outcome) modal = "outcome";
  if (!game.units.some((u) => u.id === selected && u.hp > 0))
    selected = game.units.find((u) => u.side === "allied" && u.hp > 0)?.id;
  render();
}
function activate(element) {
  if (element.dataset.unit) {
    const unit = game.units.find((u) => u.id === element.dataset.unit);
    if (mode === "attack" && unit.side === "axis") {
      const attacker = game.units.find((u) => u.id === selected);
      if (!canAttack(game, attacker, unit))
        return apply({
          ok: false,
          message:
            "That enemy is not a valid target. Move adjacent with at least 2 CP and 20 ammunition remaining.",
        });
      targetId = unit.id;
      modal = "combat";
    } else {
      selected = unit.id;
      mode = "inspect";
      messageError = false;
      message =
        unit.side === "allied"
          ? `${unit.name} selected. Choose an order in the formation panel.`
          : `${unit.name} spotted. Enemy formations are controlled by the Axis commander.`;
    }
    render();
    return;
  }
  if (element.dataset.hex) {
    const [c, r] = element.dataset.hex.split(",").map(Number);
    apply(moveUnit(game, selected, { c, r }));
    return;
  }
  const action = element.dataset.action;
  switch (action) {
    case "help":
      modal = "help";
      break;
    case "reset":
      modal = "reset";
      break;
    case "outcome":
      modal = "outcome";
      break;
    case "close":
      modal = null;
      break;
    case "confirm-reset":
      game = newGame();
      selected = "7arm";
      mode = "inspect";
      modal = null;
      zoom = 1;
      messageError = false;
      message = "New orders received. Advance west and secure Tobruk.";
      break;
    case "grid":
      grid = !grid;
      break;
    case "supplies":
      supplies = !supplies;
      break;
    case "zoom-in":
      zoom = Math.min(1.75, zoom + 0.25);
      break;
    case "zoom-out":
      zoom = Math.max(1, zoom - 0.25);
      break;
    case "zoom-reset":
      zoom = 1;
      break;
    case "cancel":
      mode = "inspect";
      break;
    case "move":
      mode = mode === "move" ? "inspect" : "move";
      break;
    case "attack":
      mode = mode === "attack" ? "inspect" : "attack";
      break;
    case "digin":
      return apply(digIn(game, selected));
    case "resupply":
      return apply(resupply(game, selected));
    case "confirm-attack":
      modal = null;
      mode = "inspect";
      return apply(attack(game, selected, targetId));
    case "end":
      mode = "inspect";
      return apply(endStage(game));
  }
  render();
}
app.addEventListener("click", (event) => {
  const element = event.target.closest(
    "[data-action], [data-unit], [data-hex]",
  );
  if (element && !element.disabled) activate(element);
});
app.addEventListener("keydown", (event) => {
  if (
    (event.key === "Enter" || event.key === " ") &&
    event.target.matches('g[role="button"]')
  ) {
    event.preventDefault();
    activate(event.target);
  }
  if (event.key === "Escape" && !modal && mode !== "inspect") {
    mode = "inspect";
    render();
  }
});
render();
