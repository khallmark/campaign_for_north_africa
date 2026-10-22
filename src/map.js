import {
  COAST,
  HEXES,
  TOBRUK,
  DEPOT,
  position,
  terrain,
  hexKey,
  reachable,
  canAttack,
  distance,
} from "./game.js";

export const sprite = (unit) =>
  `/sprites/${unit.type === "tank" ? (unit.side === "axis" ? "panzer" : "crusader") : unit.type}.png`;
const points = (x, y, radius = 41.57) =>
  Array.from({ length: 6 }, (_, i) => {
    const angle = ((60 * i - 30) * Math.PI) / 180;
    return `${x + radius * Math.cos(angle)},${y + radius * Math.sin(angle)}`;
  }).join(" ");

export function renderMap(state, selected, { mode, grid, supplies, zoom }) {
  const active = state.units.find((u) => u.id === selected && u.hp > 0);
  const moves = mode === "move" ? reachable(state, active) : new Map();
  const coast = COAST.map((p) => p.join(",")).join(" ");
  const center = active ? position(active) : { x: 560, y: 330 };
  const width = 1120 / zoom;
  const height = 660 / zoom;
  const left = Math.max(0, Math.min(1120 - width, center.x - width / 2));
  const top = Math.max(0, Math.min(660 - height, center.y - height / 2));
  const truck = state.units.find((u) => u.id === "supply" && u.hp > 0);
  return `<svg class="battle-map ${grid ? "" : "no-grid"}" viewBox="${left} ${top} ${width} ${height}" aria-label="Interactive tactical map of the Tobruk frontier" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <pattern id="sand" width="110" height="90" patternUnits="userSpaceOnUse"><path d="M10 20l2 1m40 30l2-1m37 25l1 1M75 12l2 0M23 70l1-2" stroke="#a58f64" stroke-width="1.2" opacity=".34"/><path d="M35 38q9-4 19-1m23 17q8-4 14-1" stroke="#c2ac7d" stroke-width=".6" fill="none" opacity=".35"/></pattern>
      <pattern id="waves" width="55" height="35" patternUnits="userSpaceOnUse"><path d="M4 18q7-3 15 0t15 0" stroke="#c6d2cc" stroke-width=".7" fill="none" opacity=".3"/></pattern>
      <filter id="paper"><feTurbulence type="fractalNoise" baseFrequency=".64" numOctaves="3" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/><feComponentTransfer><feFuncA type="linear" slope=".055"/></feComponentTransfer><feBlend in="SourceGraphic" mode="multiply"/></filter>
      <filter id="unit-shadow" x="-40%" y="-40%" width="180%" height="200%"><feDropShadow dx="0" dy="3" stdDeviation="3" flood-color="#473d29" flood-opacity=".3"/></filter>
      <clipPath id="land"><polygon points="0,660 0,173 ${coast} 1120,660"/></clipPath>
    </defs>
    <rect width="1120" height="660" fill="#a8bfc0"/>
    <rect width="1120" height="660" fill="url(#waves)"/>
    <polygon points="0,660 0,173 ${coast} 1120,660" fill="#e2d3af"/>
    <g clip-path="url(#land)">
      <path d="M-70 370Q140 230 390 370T790 450T1180 440M-80 390Q140 250 390 390T790 470T1180 460M-80 410Q140 270 390 410T790 490T1180 480" stroke="#c4b285" stroke-width="1.1" opacity=".4" fill="none"/>
      <path d="M-100 550Q200 420 360 540T720 560T1150 610M-100 560Q200 430 360 550T720 570T1150 620M-100 570Q200 440 360 560T720 580T1150 630" stroke="#c4b285" stroke-width=".8" opacity=".4" fill="none"/>
      <rect width="1120" height="660" fill="url(#sand)"/>
      <g class="hex-grid">${HEXES.map((h) => {
        const { x, y } = position(h);
        return `<polygon points="${points(x, y)}"/>`;
      }).join("")}</g>
      <path d="M-10 219L116 226L187 220L296 235L366 236L441 270L517 283L595 314L664 350L723 351L790 413L865 413L944 405L1042 401L1130 375" stroke="#ae9670" stroke-width="5" fill="none" opacity=".65"/>
      <path d="M-10 219L116 226L187 220L296 235L366 236L441 270L517 283L595 314L664 350L723 351L790 413L865 413L944 405L1042 401L1130 375" stroke="#f3e4c1" stroke-width="2.5" fill="none"/>
      <path d="M296 235Q310 305 330 350T440 479L656 479L764 541L1016 479" stroke="#ae9670" stroke-width="1.7" stroke-dasharray="5 5" fill="none" opacity=".6"/>
      <path d="M855 338L850 660" stroke="#97856b" stroke-width="1.4" stroke-dasharray="8 6" opacity=".65"/>
      ${HEXES.filter((h) => terrain(h) === "ridge")
        .map((h) => {
          const { x, y } = position(h);
          return `<g transform="translate(${x - 25},${y - 8})" opacity=".5" fill="#b8a478" stroke="#9b885e" stroke-width=".9"><path d="M0 20L14 0L27 20M17 23L34-5L52 23"/><path d="M14 0L17 14L23 13M34-5L36 13L42 10" fill="none"/></g>`;
        })
        .join("")}
    </g>
    <polyline points="${coast}" stroke="#f6e8c6" stroke-width="5" fill="none"/>
    <polyline points="${coast}" stroke="#8fa9a7" stroke-width="1.2" fill="none"/>
    <rect width="1120" height="660" fill="transparent" filter="url(#paper)" opacity=".45" pointer-events="none"/>
    <g class="map-lettering" pointer-events="none">
      <text x="576" y="128" class="sea-name" text-anchor="middle" transform="rotate(8 576 128)">Mediterranean Sea</text>
      <text x="80" y="70" class="region-name">CYRENAICA</text><text x="80" y="90" class="region-caption">Eastern Libya · Western Egypt</text>
      <text x="180" y="564" class="desert-name">L I B Y A N   D E S E R T</text>
      <text x="819" y="613" class="country-name" transform="rotate(-90 819 613)">LIBYA</text>
      <text x="879" y="613" class="country-name" transform="rotate(-90 879 613)">EGYPT</text>
      <text x="448" y="291" class="road-name" transform="rotate(21 448 291)">Via Balbia</text>
      <g transform="translate(297 190)"><text text-anchor="middle" class="town-name">Tobruk</text><text y="15" text-anchor="middle" class="objective-label">OBJECTIVE</text></g>
      <g transform="translate(652 305)"><circle cy="14" r="3"/><text text-anchor="middle" class="town-name">Bardia</text></g>
      <g transform="translate(842 386)"><circle cx="-15" cy="-4" r="3"/><text class="town-name">Sollum</text></g>
      <g transform="translate(493 491)"><text class="small-town">Sidi Rezegh</text></g>
      <g transform="translate(917 461)"><text class="small-town">Frontier depot</text></g>
      <g transform="translate(1016 479)"><rect x="-12" y="-10" width="24" height="20" fill="#f1e5c6" stroke="#68745a"/><path d="M-12-10L0-18L12-10M-5 10V0H5V10" fill="none" stroke="#68745a" stroke-width="2"/></g>
    </g>
    ${
      supplies
        ? `<g class="supply-overlay">${HEXES.filter(
            (h) =>
              distance(h, DEPOT) <= 2 || (truck && distance(h, truck) <= 2),
          )
            .map((h) => {
              const { x, y } = position(h);
              return `<polygon points="${points(x, y, 40)}"/>`;
            })
            .join("")}</g>`
        : ""
    }
    <g class="move-targets">${[...moves]
      .map(([key, cost]) => {
        const [c, r] = key.split(",").map(Number);
        const { x, y } = position({ c, r });
        return `<g role="button" tabindex="0" data-hex="${key}" aria-label="Move to hex ${c + 1}:${r + 1}, ${cost} CP"><polygon points="${points(x, y, 39)}"/><text x="${x}" y="${y + 4}" text-anchor="middle">${cost}</text></g>`;
      })
      .join("")}</g>
    <g pointer-events="none"><circle cx="${position(TOBRUK).x}" cy="${position(TOBRUK).y}" r="33" fill="none" stroke="#a57e3b" stroke-width="2" stroke-dasharray="4 4"/></g>
    ${state.units
      .filter((u) => u.hp > 0)
      .map((u) => {
        const { x, y } = position(u);
        const isSelected = u.id === selected;
        const target = mode === "attack" && canAttack(state, active, u);
        return `<g class="map-unit ${u.side} ${isSelected ? "selected" : ""} ${target ? "targetable" : ""}" role="button" tabindex="0" aria-label="${u.name}, ${u.side === "allied" ? "Commonwealth" : "Axis"}, strength ${u.hp}%" aria-pressed="${isSelected}" data-unit="${u.id}" transform="translate(${x} ${y})">
        <title>${u.name} · ${u.hp}% strength · ${u.cp} CP${u.entrenched ? " · Dug in" : ""}</title>
        <ellipse class="selection-halo" rx="39" ry="29" cy="3"/>
        ${u.entrenched ? '<path d="M-34 8L-34 20L-18 28H18L34 20V8" fill="none" stroke="#605a3d" stroke-width="4"/>' : ""}
        <image href="${sprite(u)}" x="-39" y="-37" width="78" height="60" filter="url(#unit-shadow)" class="unit-sprite"/>
        <rect class="counter" x="-26" y="16" width="52" height="19" rx="3"/>
        <text class="counter-text" x="0" y="29" text-anchor="middle">${u.short}</text>
        <rect x="-23" y="35" width="46" height="3" rx="1" fill="#685b45" opacity=".3"/>
        <rect x="-23" y="35" width="${(46 * u.hp) / 100}" height="3" rx="1" fill="${u.side === "allied" ? "#546a46" : "#a15c45"}"/>
        ${target ? '<circle r="32" fill="none" stroke="#a04c38" stroke-width="2" stroke-dasharray="4 3"/>' : ""}
      </g>`;
      })
      .join("")}
    <g transform="translate(1030 91)" class="compass" pointer-events="none"><text y="-32" text-anchor="middle">N</text><path d="M0-22L7 0L0 22L-7 0Z" fill="#4c6665"/><path d="M-22 0L0-7L22 0L0 7Z" fill="#d5dfd5"/><circle r="3" fill="#4c6665"/></g>
  </svg>`;
}
