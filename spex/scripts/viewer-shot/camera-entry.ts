/** Bundle entry for `kick.mjs`. See `show-entry.ts` for why these exist.
 *
 * Unlike the M62 entry this one pulls three.js in with it, because
 * `CameraDirector` holds `Vector3`/`Vector2` scratch. That means a second
 * copy of three on the page alongside the viewer's own — which is fine here
 * and would not be anywhere else: the director never constructs anything the
 * viewer has to recognise, it only reads and writes numbers on the camera
 * object it is handed.
 */

import { CameraDirector, freeCameraFromUrl, NEAR_FACTOR, FAR_FACTOR } from '../../viewer/src/show/camera';

(globalThis as unknown as { __spexCamera: unknown }).__spexCamera = {
  CameraDirector,
  freeCameraFromUrl,
  NEAR_FACTOR,
  FAR_FACTOR,
};
