/** LDraw colours -> three.js materials.
 *
 * The manifest's colours are already **linear** (see `bundle.ts`), so they go
 * into `Color.setRGB(..., LinearSRGBColorSpace)` verbatim. Passing them
 * through any sRGB conversion here would brighten every material by roughly
 * 2.2x — the single easiest way to make real LDraw colours look wrong.
 *
 * M54 deliberately ships only baseColor + roughness. The full finish table
 * (clearcoat for the ABS skin layer, transmission and ior for transparent
 * parts, iridescence for pearlescent, the metallic tiers) is M56's job, and
 * those numbers are calibrated artistic choices rather than measurements —
 * they belong in one place, with that milestone's reasoning attached.
 */

import * as THREE from 'three';
import type { MeshBundle, MeshMaterial, MeshPart } from './bundle';

/** Opaque ABS. Not a measurement: a calibrated choice, from the technical-art
 * review in `docs/FUGEN-ENGINE-REVIEW-01.md`. */
const ABS_ROUGHNESS = 0.34;
/** Black reads *only* by its specular highlight — its diffuse term is nearly
 * zero, so at 0.34 a black brick is an unreadable silhouette. */
const BLACK_ROUGHNESS = 0.22;
const LDRAW_BLACK = 0;

function linearColor(rgb: [number, number, number]): THREE.Color {
  return new THREE.Color().setRGB(rgb[0], rgb[1], rgb[2], THREE.LinearSRGBColorSpace);
}

/** Builds and caches one three.js material per LDraw colour, and resolves a
 * part's submeshes against a given instance colour.
 *
 * A part's geometry carries no colour of its own (M51 keeps LDraw code 16
 * unresolved), which is exactly what lets one mesh serve every colour it is
 * placed in — the basis of M55's instancing. A submesh whose `material` is
 * `null` means "inherit"; anything else is a fixed accent colour molded into
 * the part itself, and stays that colour no matter how the instance is
 * coloured. */
export class MaterialLibrary {
  private readonly materials: MeshMaterial[];
  private readonly cache = new Map<number, THREE.MeshStandardMaterial>();

  constructor(bundle: MeshBundle) {
    this.materials = bundle.materials;
  }

  /** The three.js material for one LDraw colour index, created once. */
  get(index: number): THREE.MeshStandardMaterial {
    const cached = this.cache.get(index);
    if (cached) return cached;
    const entry = this.materials[index];
    if (!entry) throw new Error(`mesh bundle references material ${index}, which it does not define`);
    const material = new THREE.MeshStandardMaterial({
      color: linearColor(entry.baseColor),
      roughness: entry.colorCode === LDRAW_BLACK ? BLACK_ROUGHNESS : ABS_ROUGHNESS,
      metalness: 0.0,
      // On, and correct only because M51 fixed BFC winding and M52 reversed
      // it back at the mirror. If an interior surface is ever visible here,
      // the bug is upstream of this line.
      side: THREE.FrontSide,
    });
    material.name = entry.name;
    this.cache.set(index, material);
    return material;
  }

  /** LDraw's own edge colour for a colour index, linear. Unused until M57's
   * line pass, but it belongs with the material it describes. */
  edgeColor(index: number): THREE.Color {
    const entry = this.materials[index];
    if (!entry) throw new Error(`mesh bundle references material ${index}, which it does not define`);
    return linearColor(entry.edgeColor);
  }

  /** Materials for one part's geometry groups, in submesh order, with
   * `null` submeshes resolved to the instance's own colour. */
  resolve(part: MeshPart, instanceMaterial: number): THREE.MeshStandardMaterial[] {
    return part.submeshes.map((s) => this.get(s.material ?? instanceMaterial));
  }

  dispose() {
    for (const m of this.cache.values()) m.dispose();
    this.cache.clear();
  }

  get count(): number {
    return this.cache.size;
  }
}
