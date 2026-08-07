/** M62 — the TypeScript view of `show-resolved.json`.
 *
 * A hand-written mirror of `spec/show-resolved.schema.json`, the same way
 * `mesh/bundle.ts` mirrors `mesh.schema.json`. Generating these from the
 * schema would be one more build step to keep working, and the schema is
 * validated against real `spex show-build` output by
 * `crates/spex-cli/tests/schema_validation.rs` — so the schema is the
 * authority and this file is a reader of it.
 *
 * Everything here is already absolute: seconds rather than normalised time,
 * instance indices rather than globs, an integer repeat count rather than a
 * range. That is the whole point of the resolved format — a player that has
 * to work anything out per frame is a player that can drift.
 */

export const RESOLVED_FORMAT_VERSION = 1;

export type Vec3 = [number, number, number];
export type Quat = [number, number, number, number];

export interface ResolvedTempo {
  bpm: number;
  beatsPerBar: number;
  barSeconds: number;
  beatSeconds: number;
}

export interface ResolvedScene {
  id: string;
  prefix: string;
  /** Relative to the show directory, e.g. `bundles/monolith`. */
  bundle: string;
  instanceCount?: number;
}

export interface TargetBinding {
  /** The source glob, kept for the HUD and for diffing. Not used to select. */
  glob: string;
  scene?: string;
  /** Absent only when the show was built with `--no-bundles`. */
  instances?: number[];
}

export type EasingName =
  | 'linear'
  | 'quadIn'
  | 'quadOut'
  | 'quadInOut'
  | 'cubicIn'
  | 'cubicOut'
  | 'cubicInOut'
  | 'expoIn'
  | 'expoOut'
  | 'step';

export interface ResolvedKey<T> {
  timeSec: number;
  value: T;
  easing: EasingName;
}

export interface TransformValue {
  position?: Vec3;
  /** Euler XYZ in degrees. Carried rather than converted: a quaternion pair
   * cannot express more than a half-turn, and A1-S03 turns a full one. */
  rotation?: Vec3;
  quaternion?: Quat;
  scale?: number;
}

export interface CameraValue {
  position?: Vec3;
  lookAt?: Vec3;
  fovDeg?: number;
}

export type MaterialProperty =
  | 'opacity'
  | 'roughness'
  | 'metalness'
  | 'emissiveIntensity'
  | 'transmission'
  | 'edgeOpacity';

export type PostProperty =
  | 'exposure'
  | 'bloomThreshold'
  | 'bloomStrength'
  | 'bloomRadius'
  | 'vignette'
  | 'gradeStrength'
  | 'grain';

export type ResolvedTrack =
  | { kind: 'transform'; target: TargetBinding; keys: ResolvedKey<TransformValue>[] }
  | { kind: 'dissolve'; target: TargetBinding; keys: ResolvedKey<number>[] }
  | { kind: 'material'; target: TargetBinding; property: MaterialProperty; keys: ResolvedKey<number>[] }
  | { kind: 'post'; property: PostProperty; keys: ResolvedKey<number>[] }
  | { kind: 'hud'; element: string; keys: ResolvedKey<number>[] }
  | { kind: 'pointCloud'; target: TargetBinding; keys: ResolvedKey<number>[] };

export type CameraMode = 'keyed' | 'orbit' | 'dolly' | 'exponentialZoom';

export interface ResolvedCameraTrack {
  mode: CameraMode;
  fovDeg?: number;
  keys?: ResolvedKey<CameraValue>[];
  orbit?: { center: Vec3; radius: number; height: number; startDeg: number; endDeg: number };
  dolly?: { from: Vec3; to: Vec3; lookAt: Vec3 };
  exponentialZoom?: { from: number; to: number; lookAt: Vec3; direction?: Vec3 };
  motionBlur?: number;
}

export type CueKind = 'audio' | 'hud' | 'seed' | 'marker';

export interface ResolvedCue {
  timeSec: number;
  kind: CueKind;
  shotId: string;
  payload?: Record<string, unknown>;
}

export interface ResolvedShot {
  id: string;
  title: string;
  movementId: string;
  movementTitle?: string;
  romanNumeral?: string;
  tier: 1 | 2 | 3;
  startSec: number;
  endSec: number;
  durationSec: number;
  startBar?: number;
  durationBars?: number;
  repeatCount?: number;
  scenes?: string[];
  camera: ResolvedCameraTrack;
  tracks?: ResolvedTrack[];
  cues?: ResolvedCue[];
  note?: string;
}

export interface ResolvedShow {
  version: number;
  generator: string;
  id: string;
  title: string;
  subtitle?: string;
  archiveSignature: string;
  tempo: ResolvedTempo;
  targetSec: number;
  durationSec: number;
  beatAligned: boolean;
  endless: boolean;
  seed: number;
  palette?: Record<string, Vec3>;
  scenes: ResolvedScene[];
  shots: ResolvedShot[];
  credits?: { lines?: string[]; required?: string[] };
}

/** Fetches `show-resolved.json`, or `null` when there is none.
 *
 * The same one-branch mode switch `fetchMeshBundle` uses: absence is how the
 * viewer decides which thing it is looking at, so a 404 here is a fact and
 * not an error. */
export async function fetchResolvedShow(baseUrl: string): Promise<ResolvedShow | null> {
  const res = await fetch(`${baseUrl.replace(/\/$/, '')}/show-resolved.json`);
  if (!res.ok) return null;
  const show = (await res.json()) as ResolvedShow;
  if (show.version !== RESOLVED_FORMAT_VERSION) {
    throw new Error(
      `show-resolved.json is version ${show.version}, this viewer reads ${RESOLVED_FORMAT_VERSION}. ` +
        `Rebuild it with \`spex show-build\`.`,
    );
  }
  return show;
}
