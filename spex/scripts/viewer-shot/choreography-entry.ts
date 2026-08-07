/** Bundle entry for `assembly.mjs`. See `show-entry.ts` for why these exist. */

import {
  AssemblyChoreography,
  startOffsetLdu,
  startOffsetMm,
  staggeredProgress,
} from '../../viewer/src/show/choreography';

(globalThis as unknown as { __spexChoreo: unknown }).__spexChoreo = {
  AssemblyChoreography,
  startOffsetLdu,
  startOffsetMm,
  staggeredProgress,
};
