import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { fetchTileset, fetchNodePoints, fetchNodeLabels, fetchGraphMeta, type Bounds, type NodeLabel } from './tileset';
import { NodeIndex, selectNodes } from './lod';
import { buildFullSweepPath } from './packetAnimation';

/** In gallery mode (`spex gallery`, or a static export served by e.g. GitHub
 * Pages) each demo lives under `.../d/<name>/`, with its tileset at
 * `.../d/<name>/tileset`; single-tileset mode (`spex serve`) serves it at
 * the root, under plain `/tileset`. Same bundle, either way.
 *
 * Paths here are deliberately relative, not root-absolute: a static export
 * can be hosted at a domain root or under a project-pages subpath
 * (`username.github.io/reponame/...`), and relative fetches/links resolve
 * correctly against the current document's location regardless of how deep
 * that prefix is — no need to know it in advance. The regex also has no `^`
 * anchor for the same reason: it just needs to find `/d/<name>/` at the end
 * of the pathname, wherever it's mounted. */
const GALLERY_MATCH = window.location.pathname.match(/\/d\/([^/]+)\/?$/);
const CURRENT_DEMO_NAME = GALLERY_MATCH ? GALLERY_MATCH[1] : null;
const TILESET_BASE = CURRENT_DEMO_NAME ? 'tileset' : '/tileset';

/** "Demoscene" screensaver mode (`?cycle=1`, only meaningful in gallery mode):
 * auto-rotates the camera and, after a while, jumps to a random other demo.
 * Reuses the gallery's own front page as the list of what's available —
 * no separate API endpoint needed. */
const CYCLE_MODE = CURRENT_DEMO_NAME !== null && new URLSearchParams(window.location.search).has('cycle');
const CYCLE_INTERVAL_MS = 20_000;
const cycleIndicatorEl = document.getElementById('cycle-indicator') as HTMLDivElement;
const cycleCountdownEl = document.getElementById('cycle-countdown') as HTMLSpanElement;

async function goToRandomOtherDemo() {
  try {
    // Relative: from `.../d/<current>/`, up two levels reaches the gallery
    // root regardless of any hosting subpath prefix.
    const html = await (await fetch('../../')).text();
    const names = [...html.matchAll(/href="d\/([^/"]+)\/"/g)].map((m) => m[1]);
    const others = names.filter((n) => n !== CURRENT_DEMO_NAME);
    const pool = others.length > 0 ? others : names;
    if (pool.length === 0) return;
    const next = pool[Math.floor(Math.random() * pool.length)];
    window.location.href = `../${next}/?cycle=1`;
  } catch (err) {
    console.error('cycle: failed to find another demo', err);
  }
}

const statusEl = document.getElementById('status') as HTMLDivElement;
const hudEl = document.getElementById('hud') as HTMLDivElement;
const debugPanelEl = document.getElementById('debug-panel') as HTMLDivElement;
const labelsEl = document.getElementById('labels') as HTMLDivElement;
const pointSizeInput = document.getElementById('pointSize') as HTMLInputElement;
const pointBudgetInput = document.getElementById('pointBudget') as HTMLInputElement;
const showLabelsInput = document.getElementById('showLabels') as HTMLInputElement;
const animatePacketInput = document.getElementById('animatePacket') as HTMLInputElement;
const showEdgesInput = document.getElementById('showEdges') as HTMLInputElement;
const autoRotateInput = document.getElementById('autoRotate') as HTMLInputElement;
const graphMetaEl = document.getElementById('graph-meta') as HTMLDivElement;
const graphTitleEl = document.getElementById('graph-title') as HTMLDivElement;
const graphLegendEl = document.getElementById('graph-legend') as HTMLDivElement;
const legendMinEl = document.getElementById('legend-min') as HTMLSpanElement;
const legendMaxEl = document.getElementById('legend-max') as HTMLSpanElement;
const legendCaptionEl = document.getElementById('legend-caption') as HTMLDivElement;

/** Compact one-line rendering of a metadata value — mirrors
 * `spex_graph::display::compact_value` (long arrays collapse to a count) so
 * the browser tooltip and the terminal `graph-print` view agree. */
function formatMetadataValue(v: unknown): string {
  if (Array.isArray(v)) {
    return v.length > 3 ? `[${v.length} items]` : `[${v.join(', ')}]`;
  }
  return String(v);
}

/** Full multi-line tooltip: label, metric (with its unit from meta.json),
 * then every metadata field — the same detail `graph-print` already shows
 * in the terminal, now also visible in the browser instead of being thrown
 * away after just `label (metric)`. */
function buildTooltipText(n: NodeLabel, metricLabel: string | null): string {
  const lines: string[] = [n.label];
  if (n.metric != null) {
    lines.push(metricLabel ? `${n.metric.toFixed(2)} ${metricLabel}` : n.metric.toFixed(2));
  }
  for (const [key, value] of Object.entries(n.metadata)) {
    lines.push(`${key}: ${formatMetadataValue(value)}`);
  }
  return lines.join('\n');
}

function boundsCenter(b: Bounds): [number, number, number] {
  return [(b.min[0] + b.max[0]) / 2, (b.min[1] + b.max[1]) / 2, (b.min[2] + b.max[2]) / 2];
}

function boundsDiagonal(b: Bounds): number {
  const dx = b.max[0] - b.min[0];
  const dy = b.max[1] - b.min[1];
  const dz = b.max[2] - b.min[2];
  return Math.hypot(dx, dy, dz) || 1;
}

async function main() {
  const tileset = await fetchTileset(TILESET_BASE).catch((err: Error) => {
    statusEl.textContent = `failed to load tileset: ${err.message}`;
    throw err;
  });
  statusEl.textContent = `${tileset.pointCount.toLocaleString()} points across ${tileset.nodes.length} nodes`;

  const index = new NodeIndex(tileset);

  // Optional: whole-graph description (absent for plain point-cloud tilesets)
  // — a persistent header/legend so a viewer doesn't have to guess what
  // they're looking at or hunt for a hover tooltip to find out.
  const graphMeta = await fetchGraphMeta(TILESET_BASE);
  if (graphMeta) {
    graphMetaEl.style.display = 'block';
    graphTitleEl.textContent = graphMeta.title ?? `${graphMeta.nodeCount} nodes`;
    if (graphMeta.metricLabel && graphMeta.metricMin != null && graphMeta.metricMax != null) {
      graphLegendEl.style.display = 'flex';
      legendMinEl.textContent = graphMeta.metricMin.toFixed(1);
      legendMaxEl.textContent = graphMeta.metricMax.toFixed(1);
      legendCaptionEl.textContent = graphMeta.metricLabel;
    } else {
      graphLegendEl.style.display = 'none';
      legendCaptionEl.textContent = '';
    }
  }

  // Optional: node labels (absent for plain point-cloud tilesets from `spex convert`).
  const nodeLabels = await fetchNodeLabels(TILESET_BASE);
  const labelEls = new Map<string, HTMLDivElement>();
  for (const n of nodeLabels) {
    const el = document.createElement('div');
    el.className = 'node-label';
    el.textContent = buildTooltipText(n, graphMeta?.metricLabel ?? null);
    labelsEl.appendChild(el);
    labelEls.set(n.id, el);
  }
  // Only the node nearest the cursor gets a visible tooltip — with many nodes,
  // showing every label at once produces unreadable overlapping text.
  const HOVER_RADIUS_PX = 40;
  let mouseX = -Infinity;
  let mouseY = -Infinity;

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0x0b0e12);

  const diag = boundsDiagonal(tileset.bounds);
  const center = boundsCenter(tileset.bounds);

  // Optional: crisp real line edges between each node and its real parent,
  // layered on top of the existing dim point-trail edges (baked into the
  // tileset's points at graph-layout time — replacing those is a separate,
  // riskier change since every existing demo's point count depends on them;
  // this is purely additive) for a clearer sense of tree structure at a
  // glance. Absent for plain point-cloud tilesets (no nodeLabels).
  if (nodeLabels.length > 0) {
    const byId = new Map(nodeLabels.map((n) => [n.id, n]));
    const positions: number[] = [];
    for (const n of nodeLabels) {
      if (n.parent === null) continue;
      const parent = byId.get(n.parent);
      if (!parent) continue;
      positions.push(parent.center[0], parent.center[1], parent.center[2], n.center[0], n.center[1], n.center[2]);
    }
    if (positions.length > 0) {
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
      const material = new THREE.LineBasicMaterial({ color: 0xffffff, transparent: true, opacity: 0.45 });
      const edgeLines = new THREE.LineSegments(geometry, material);
      edgeLines.visible = showEdgesInput.checked;
      scene.add(edgeLines);
      showEdgesInput.addEventListener('input', () => {
        edgeLines.visible = showEdgesInput.checked;
      });
    }
  }

  // Optional: animate a marker sweeping the full tree (a real depth-first
  // traversal, heaviest subtree first) — absent/no-op for plain point-cloud
  // tilesets with no node labels at all.
  const packetPath = buildFullSweepPath(nodeLabels);
  const packetSpeed = diag * 0.15; // units/sec — a hop's travel time scales with its real distance
  let packetMesh: THREE.Mesh | null = null;
  let packetSegment = 0;
  let packetT = 0;
  const packetA = new THREE.Vector3();
  const packetB = new THREE.Vector3();
  const packetHitProjection = new THREE.Vector3();
  // "Hit" flash: briefly show the same hover tooltip (label/metric/metadata)
  // for whichever node the packet just reached, so the metric view isn't
  // only reachable by mousing over a blob — the traveling packet surfaces it too.
  const PACKET_HIT_FLASH_SECONDS = 1.2;
  let packetHitNode: NodeLabel | null = null;
  let packetHitTimer = 0;
  if (packetPath.length >= 2) {
    const geometry = new THREE.SphereGeometry(Math.max(diag * 0.01, 0.001), 16, 16);
    const material = new THREE.MeshBasicMaterial({ color: 0xffffff });
    packetMesh = new THREE.Mesh(geometry, material);
    packetMesh.position.set(packetPath[0].center[0], packetPath[0].center[1], packetPath[0].center[2]);
    packetMesh.visible = animatePacketInput.checked;
    scene.add(packetMesh);
  }
  animatePacketInput.addEventListener('input', () => {
    if (packetMesh) packetMesh.visible = animatePacketInput.checked;
    if (!animatePacketInput.checked) packetHitTimer = 0;
  });

  function updatePacket(deltaSeconds: number) {
    packetHitTimer = Math.max(0, packetHitTimer - deltaSeconds);
    if (!packetMesh || !animatePacketInput.checked) return;
    const numSegments = packetPath.length - 1;
    let a = packetPath[packetSegment];
    let b = packetPath[packetSegment + 1];
    packetA.set(a.center[0], a.center[1], a.center[2]);
    packetB.set(b.center[0], b.center[1], b.center[2]);
    const segmentLength = packetA.distanceTo(packetB) || 0.001;
    const segmentDuration = segmentLength / packetSpeed;
    packetT += deltaSeconds / segmentDuration;
    if (packetT >= 1) {
      packetT = 0;
      packetSegment = (packetSegment + 1) % numSegments; // loops back to the start immediately, no return trip
      a = packetPath[packetSegment];
      b = packetPath[packetSegment + 1];
      packetA.set(a.center[0], a.center[1], a.center[2]);
      packetB.set(b.center[0], b.center[1], b.center[2]);
      packetHitNode = a; // the node the packet just reached
      packetHitTimer = PACKET_HIT_FLASH_SECONDS;
    }
    packetMesh.position.lerpVectors(packetA, packetB, packetT);
    const pulse = 1 + 0.15 * Math.sin(performance.now() / 150);
    packetMesh.scale.setScalar(pulse);
  }

  // Reuses the same tooltip elements/positioning as the mouse-hover labels
  // (see updateLabels below) — just driven by the packet's arrival instead
  // of cursor proximity, and shown regardless of where the mouse is.
  function updatePacketHitLabel() {
    if (!showLabelsInput.checked || packetHitTimer <= 0 || !packetHitNode) return;
    const el = labelEls.get(packetHitNode.id);
    if (!el) return;
    camera.updateMatrixWorld();
    packetHitProjection.set(packetHitNode.center[0], packetHitNode.center[1], packetHitNode.center[2]).project(camera);
    if (packetHitProjection.z < -1 || packetHitProjection.z > 1) return; // behind the camera
    el.style.display = 'block';
    el.style.left = `${(packetHitProjection.x * 0.5 + 0.5) * window.innerWidth}px`;
    el.style.top = `${(-packetHitProjection.y * 0.5 + 0.5) * window.innerHeight}px`;
  }

  // A lower-left "debug panel" for complex (chain/packet) demos — a plain-
  // language readout of what's happening right now, so a viewer doesn't
  // have to hover a blob or guess to follow along.
  function updateDebugPanel() {
    if (!packetMesh || packetPath.length < 2 || !animatePacketInput.checked) {
      debugPanelEl.style.display = 'none';
      return;
    }
    const a = packetPath[packetSegment];
    const b = packetPath[packetSegment + 1] ?? packetPath[0];
    const pct = Math.round(packetT * 100);
    const lines = [`packet: ${a.label} -> ${b.label}  (${pct}%)`, `hop ${packetSegment + 1}/${packetPath.length - 1}`];
    const metadata = b.metadata;
    if (typeof metadata.status === 'string') lines.push(`status: ${metadata.status}`);
    if (typeof metadata.distanceKm === 'number') lines.push(`distance: ${metadata.distanceKm} km`);
    if (b.metric != null) {
      lines.push(graphMeta?.metricLabel ? `${b.metric.toFixed(2)} ${graphMeta.metricLabel}` : b.metric.toFixed(2));
    }
    debugPanelEl.textContent = lines.join('\n');
    debugPanelEl.style.display = 'block';
  }

  const camera = new THREE.PerspectiveCamera(60, window.innerWidth / window.innerHeight, Math.max(diag / 10000, 0.001), diag * 10);
  camera.position.set(center[0] + diag * 0.6, center[1] + diag * 0.6, center[2] + diag * 0.6);
  camera.lookAt(center[0], center[1], center[2]);

  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(window.innerWidth, window.innerHeight);
  document.getElementById('app')!.prepend(renderer.domElement);

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.target.set(center[0], center[1], center[2]);
  controls.enableDamping = true;
  controls.dampingFactor = 0.08;
  if (CYCLE_MODE) {
    controls.autoRotate = true;
    controls.autoRotateSpeed = 1.5;
  } else {
    controls.autoRotateSpeed = 0.6; // a slow showcase spin, not three.js's brisker default
  }
  autoRotateInput.checked = controls.autoRotate;
  autoRotateInput.addEventListener('input', () => {
    controls.autoRotate = autoRotateInput.checked;
  });
  controls.update();

  let cycleDeadline = 0;
  if (CYCLE_MODE) {
    cycleIndicatorEl.style.display = 'block';
    cycleDeadline = performance.now() + CYCLE_INTERVAL_MS;
    setTimeout(() => void goToRandomOtherDemo(), CYCLE_INTERVAL_MS);
  }

  renderer.domElement.addEventListener('pointermove', (ev) => {
    mouseX = ev.clientX;
    mouseY = ev.clientY;
  });
  renderer.domElement.addEventListener('pointerleave', () => {
    mouseX = -Infinity;
    mouseY = -Infinity;
  });

  const material = new THREE.PointsMaterial({
    size: parseFloat(pointSizeInput.value),
    vertexColors: true,
    sizeAttenuation: true,
  });
  pointSizeInput.addEventListener('input', () => {
    material.size = parseFloat(pointSizeInput.value);
  });

  let pointBudget = parseFloat(pointBudgetInput.value) * 1_000_000;
  pointBudgetInput.addEventListener('input', () => {
    pointBudget = parseFloat(pointBudgetInput.value) * 1_000_000;
  });

  const loaded = new Map<string, THREE.Points>();
  const pending = new Set<string>();

  async function ensureLoaded(id: string) {
    if (loaded.has(id) || pending.has(id)) return;
    pending.add(id);
    try {
      const { positions, colors } = await fetchNodePoints(TILESET_BASE, id);
      const geometry = new THREE.BufferGeometry();
      geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
      geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3, true));
      const points = new THREE.Points(geometry, material);
      scene.add(points);
      loaded.set(id, points);
    } catch (err) {
      console.error(`failed to load node ${id}`, err);
    } finally {
      pending.delete(id);
    }
  }

  function unload(id: string) {
    const points = loaded.get(id);
    if (!points) return;
    scene.remove(points);
    points.geometry.dispose();
    loaded.delete(id);
  }

  let lastSelected: Set<string> = new Set();

  function updateLOD() {
    const selected = selectNodes(index, camera, pointBudget);
    for (const id of selected) {
      if (!loaded.has(id)) void ensureLoaded(id);
    }
    for (const id of loaded.keys()) {
      if (!selected.has(id)) unload(id);
    }
    lastSelected = selected;
  }

  function updateHud(fps: number) {
    let renderedPoints = 0;
    for (const id of lastSelected) renderedPoints += index.get(id)?.pointCount ?? 0;
    hudEl.innerHTML = `
      <div>${renderedPoints.toLocaleString()} / ${tileset.pointCount.toLocaleString()} points</div>
      <div>${lastSelected.size} nodes visible &middot; ${loaded.size} loaded</div>
      <div>${fps.toFixed(0)} fps</div>
    `;
  }

  function updateLabels() {
    if (nodeLabels.length === 0) return;
    if (!showLabelsInput.checked) {
      for (const el of labelEls.values()) el.style.display = 'none';
      return;
    }

    camera.updateMatrixWorld();
    const v = new THREE.Vector3();
    let nearestId: string | null = null;
    let nearestDist = HOVER_RADIUS_PX;
    let nearestX = 0;
    let nearestY = 0;

    for (const n of nodeLabels) {
      v.set(n.center[0], n.center[1], n.center[2]).project(camera);
      if (v.z < -1 || v.z > 1) continue;
      const x = (v.x * 0.5 + 0.5) * window.innerWidth;
      const y = (-v.y * 0.5 + 0.5) * window.innerHeight;
      const dist = Math.hypot(x - mouseX, y - mouseY);
      if (dist < nearestDist) {
        nearestDist = dist;
        nearestId = n.id;
        nearestX = x;
        nearestY = y;
      }
    }

    for (const [id, el] of labelEls) {
      if (id === nearestId) {
        el.style.display = 'block';
        el.style.left = `${nearestX}px`;
        el.style.top = `${nearestY}px`;
      } else {
        el.style.display = 'none';
      }
    }
  }

  window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
  });

  statusEl.style.display = 'none';

  let lastTime = performance.now();
  let frames = 0;
  let fpsAccumMs = 0;
  let fps = 0;

  function animate() {
    requestAnimationFrame(animate);
    controls.update();
    updateLOD();
    updateLabels();

    const now = performance.now();
    const deltaSeconds = (now - lastTime) / 1000;
    frames++;
    fpsAccumMs += now - lastTime;
    lastTime = now;
    if (fpsAccumMs >= 250) {
      fps = (frames * 1000) / fpsAccumMs;
      frames = 0;
      fpsAccumMs = 0;
    }
    updatePacket(deltaSeconds);
    updatePacketHitLabel();
    updateDebugPanel();
    updateHud(fps);
    if (CYCLE_MODE) {
      cycleCountdownEl.textContent = Math.max(0, Math.ceil((cycleDeadline - now) / 1000)).toString();
    }

    renderer.render(scene, camera);
  }
  animate();
}

main().catch((err) => {
  console.error(err);
});
