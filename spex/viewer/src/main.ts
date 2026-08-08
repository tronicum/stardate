import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { fetchTileset, fetchNodePoints, fetchNodeLabels, fetchGraphMeta, fetchSequence, mergeBounds, type Bounds, type NodeLabel } from './tileset';
import { NodeIndex, selectNodes } from './lod';
import { buildConcurrentSweepPaths } from './packetAnimation';

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
  // Optional: a real multi-frame point-cloud animation (`spex
  // frame-sequence`, see `spex brick-assembly`) instead of one
  // static tileset — absent for every other demo. When present, `activeBase`
  // points at whichever frame is currently on screen; everything below
  // (fetchTileset/fetchNodePoints/the LOD selector) works completely
  // unchanged, since a frame is just a normal tileset at its own path.
  const sequence = await fetchSequence(TILESET_BASE);
  let activeBase = TILESET_BASE;
  let sequenceFrameIdx = 0;
  if (sequence) {
    activeBase = `${TILESET_BASE}/${sequence.frames[0]}`;
  }

  let tileset = await fetchTileset(activeBase).catch((err: Error) => {
    statusEl.textContent = `failed to load tileset: ${err.message}`;
    throw err;
  });
  statusEl.textContent = `${tileset.pointCount.toLocaleString()} points across ${tileset.nodes.length} nodes`;

  let index = new NodeIndex(tileset);

  // Optional: whole-graph description (absent for plain point-cloud tilesets)
  // — a persistent header/legend so a viewer doesn't have to guess what
  // they're looking at or hunt for a hover tooltip to find out.
  const graphMeta = await fetchGraphMeta(activeBase);
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
  const nodeLabels = await fetchNodeLabels(activeBase);
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

  let diag = boundsDiagonal(tileset.bounds);
  let center = boundsCenter(tileset.bounds);
  if (sequence) {
    // One stable camera framing across the whole sequence — a single
    // frame's own bounds legitimately differ a lot in size (e.g. scattered
    // vs. assembled), so the camera shouldn't reframe on every swap. Same
    // "one shared window across all frames" principle as `spex ascii
    // --animate`'s turntable orbit.
    const allTilesets = await Promise.all(sequence.frames.map((f) => fetchTileset(`${TILESET_BASE}/${f}`)));
    const combined = mergeBounds(allTilesets.map((t) => t.bounds));
    diag = boundsDiagonal(combined);
    center = boundsCenter(combined);
  }

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
    // Extra structural edges for genuinely shared nodes (e.g. a package
    // that's a transitive dependency of two formulas) — same "one real
    // node, no second 3D position" precedent as `parent` above: the line
    // just points back at whichever other real parent also depends on this
    // node's single existing position, it doesn't move or duplicate anything.
    for (const n of nodeLabels) {
      for (const extraParentId of n.extraParents ?? []) {
        const extraParent = byId.get(extraParentId);
        if (!extraParent) continue;
        positions.push(extraParent.center[0], extraParent.center[1], extraParent.center[2], n.center[0], n.center[1], n.center[2]);
      }
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

  // Optional: animate one marker per the root's direct children, each
  // independently sweeping only its own subtree (issue #26) — absent/no-op
  // for plain point-cloud tilesets with no node labels at all. A chain (one
  // child per node, e.g. a single top-level process or a traceroute) has
  // exactly one root child, so this degrades to exactly one marker with the
  // exact same path `buildFullSweepPath` always produced; a single-node
  // graph has zero root children, so zero markers. See
  // `packetAnimation.ts`'s `buildConcurrentSweepPaths` doc comment.
  const packetPaths = buildConcurrentSweepPaths(nodeLabels);
  // units/sec — a hop's travel time scales with its real 3D distance, same
  // as before #26. Every marker shares this one real-world speed constant,
  // so a marker sweeping a bigger subtree simply takes proportionally
  // longer to finish its own loop instead of being sped up/slowed down to
  // artificially finish in lockstep with the others — concurrent, but not
  // synchronized.
  const packetSpeed = diag * 0.15;
  // Distinct colors per marker (cycling if there are more markers than
  // colors — bounded by the layout's own MAX_CHILDREN_SHOWN fan-out cap, so
  // this never has to cycle more than once for real data): plain white
  // first, so a chain's single marker looks exactly as it did before #26.
  const PACKET_MARKER_COLORS = [0xffffff, 0x22e5ff, 0xff3df0, 0xffd166, 0x8cff66, 0xff6b6b, 0x6b8cff];
  const PACKET_HIT_FLASH_SECONDS = 1.2;

  interface PacketMarker {
    path: NodeLabel[];
    mesh: THREE.Mesh;
    segment: number;
    t: number;
    // "Hit" flash: briefly show the same hover tooltip (label/metric/metadata)
    // for whichever node this marker just reached, so the metric view isn't
    // only reachable by mousing over a blob — the traveling packet surfaces it too.
    hitNode: NodeLabel | null;
    hitTimer: number;
  }

  const packetMarkers: PacketMarker[] = packetPaths
    .filter((path) => path.length >= 2)
    .map((path, i) => {
      const geometry = new THREE.SphereGeometry(Math.max(diag * 0.01, 0.001), 16, 16);
      const material = new THREE.MeshBasicMaterial({ color: PACKET_MARKER_COLORS[i % PACKET_MARKER_COLORS.length] });
      const mesh = new THREE.Mesh(geometry, material);
      mesh.position.set(path[0].center[0], path[0].center[1], path[0].center[2]);
      mesh.visible = animatePacketInput.checked;
      scene.add(mesh);
      return { path, mesh, segment: 0, t: 0, hitNode: null, hitTimer: 0 };
    });
  animatePacketInput.addEventListener('input', () => {
    for (const marker of packetMarkers) {
      marker.mesh.visible = animatePacketInput.checked;
      if (!animatePacketInput.checked) marker.hitTimer = 0;
    }
  });

  const packetA = new THREE.Vector3();
  const packetB = new THREE.Vector3();
  const packetHitProjection = new THREE.Vector3();

  function updatePacketMarker(marker: PacketMarker, deltaSeconds: number) {
    marker.hitTimer = Math.max(0, marker.hitTimer - deltaSeconds);
    if (!animatePacketInput.checked) return;
    const numSegments = marker.path.length - 1;
    let a = marker.path[marker.segment];
    let b = marker.path[marker.segment + 1];
    packetA.set(a.center[0], a.center[1], a.center[2]);
    packetB.set(b.center[0], b.center[1], b.center[2]);
    const segmentLength = packetA.distanceTo(packetB) || 0.001;
    const segmentDuration = segmentLength / packetSpeed;
    marker.t += deltaSeconds / segmentDuration;
    if (marker.t >= 1) {
      marker.t = 0;
      marker.segment = (marker.segment + 1) % numSegments; // loops its own subtree independently, no return trip
      a = marker.path[marker.segment];
      b = marker.path[marker.segment + 1];
      packetA.set(a.center[0], a.center[1], a.center[2]);
      packetB.set(b.center[0], b.center[1], b.center[2]);
      marker.hitNode = a; // the node this marker just reached
      marker.hitTimer = PACKET_HIT_FLASH_SECONDS;
    }
    marker.mesh.position.lerpVectors(packetA, packetB, marker.t);
    const pulse = 1 + 0.15 * Math.sin(performance.now() / 150);
    marker.mesh.scale.setScalar(pulse);
  }

  function updatePacket(deltaSeconds: number) {
    for (const marker of packetMarkers) updatePacketMarker(marker, deltaSeconds);
  }

  // Reuses the same tooltip elements/positioning as the mouse-hover labels
  // (see updateLabels below) — just driven by each marker's arrival instead
  // of cursor proximity, and shown regardless of where the mouse is. Every
  // node belongs to exactly one root child's subtree (tree structure), so
  // no two markers ever fight over the same tooltip element.
  function updatePacketHitLabel() {
    if (!showLabelsInput.checked) return;
    for (const marker of packetMarkers) {
      if (marker.hitTimer <= 0 || !marker.hitNode) continue;
      const el = labelEls.get(marker.hitNode.id);
      if (!el) continue;
      camera.updateMatrixWorld();
      packetHitProjection.set(marker.hitNode.center[0], marker.hitNode.center[1], marker.hitNode.center[2]).project(camera);
      if (packetHitProjection.z < -1 || packetHitProjection.z > 1) continue; // behind the camera
      el.style.display = 'block';
      el.style.left = `${(packetHitProjection.x * 0.5 + 0.5) * window.innerWidth}px`;
      el.style.top = `${(-packetHitProjection.y * 0.5 + 0.5) * window.innerHeight}px`;
    }
  }

  // A lower-left "debug panel" for complex (chain/packet) demos — a plain-
  // language readout of what's happening right now, so a viewer doesn't
  // have to hover a blob or guess to follow along. A single marker (the
  // common chain case — traceroute, a single top-level process) keeps the
  // original detailed single-packet readout (status/distance/metric)
  // unchanged; concurrent markers get one compact line each instead, so N
  // markers don't turn the panel into a wall of text.
  function updateDebugPanel() {
    if (packetMarkers.length === 0 || !animatePacketInput.checked) {
      debugPanelEl.style.display = 'none';
      return;
    }
    if (packetMarkers.length === 1) {
      const marker = packetMarkers[0];
      const a = marker.path[marker.segment];
      const b = marker.path[marker.segment + 1] ?? marker.path[0];
      const pct = Math.round(marker.t * 100);
      const lines = [`packet: ${a.label} -> ${b.label}  (${pct}%)`, `hop ${marker.segment + 1}/${marker.path.length - 1}`];
      const metadata = b.metadata;
      if (typeof metadata.status === 'string') lines.push(`status: ${metadata.status}`);
      if (typeof metadata.distanceKm === 'number') lines.push(`distance: ${metadata.distanceKm} km`);
      if (b.metric != null) {
        lines.push(graphMeta?.metricLabel ? `${b.metric.toFixed(2)} ${graphMeta.metricLabel}` : b.metric.toFixed(2));
      }
      debugPanelEl.textContent = lines.join('\n');
    } else {
      const lines = packetMarkers.map((marker, i) => {
        const a = marker.path[marker.segment];
        const b = marker.path[marker.segment + 1] ?? marker.path[0];
        const pct = Math.round(marker.t * 100);
        return `packet ${i + 1}: ${a.label} -> ${b.label}  (${pct}%)  hop ${marker.segment + 1}/${marker.path.length - 1}`;
      });
      debugPanelEl.textContent = lines.join('\n');
    }
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
      const { positions, colors } = await fetchNodePoints(activeBase, id);
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

  // Advances a real point-cloud animation to its next frame: every
  // currently-loaded octree node belongs to the *old* frame's tileset, so
  // it's unloaded outright (a new frame's node ids can coincidentally match
  // the old ones — "r", "r0", ... — but point to completely different bin
  // data) rather than reused; `updateLOD()`'s next tick then loads whatever
  // the new frame's own octree actually needs, via the exact same
  // ensureLoaded/unload path every other demo already uses.
  async function advanceToFrame(frameName: string) {
    activeBase = `${TILESET_BASE}/${frameName}`;
    for (const id of [...loaded.keys()]) unload(id);
    pending.clear();
    tileset = await fetchTileset(activeBase);
    index = new NodeIndex(tileset);
  }

  if (sequence && sequence.frames.length > 1) {
    const frameIntervalMs = 1000 / Math.max(sequence.fps, 0.1);
    setInterval(() => {
      sequenceFrameIdx = (sequenceFrameIdx + 1) % sequence.frames.length;
      void advanceToFrame(sequence.frames[sequenceFrameIdx]);
    }, frameIntervalMs);
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
    const frameLine = sequence ? `<div>frame ${sequenceFrameIdx + 1} / ${sequence.frames.length}</div>` : '';
    hudEl.innerHTML = `
      <div>${renderedPoints.toLocaleString()} / ${tileset.pointCount.toLocaleString()} points</div>
      <div>${lastSelected.size} nodes visible &middot; ${loaded.size} loaded</div>
      <div>${fps.toFixed(0)} fps</div>
      ${frameLine}
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
