/** Bundle entry for `crossfade.mjs`. See `show-entry.ts` for why these exist. */

import {
  buildPointClouds,
  fetchPartPoints,
  PointCloudRenderer,
  SPREAD_RADII,
} from '../../viewer/src/show/points';

(globalThis as unknown as { __spexPoints: unknown }).__spexPoints = {
  buildPointClouds,
  fetchPartPoints,
  PointCloudRenderer,
  SPREAD_RADII,
};
