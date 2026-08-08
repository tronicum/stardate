import type { NodeLabel } from './tileset';

/** Builds the `parent -> children` map every sweep (single- or multi-marker)
 * walks: children sorted heaviest-`metric`-first, so a DFS always descends
 * into the biggest subtree first — shared by `buildFullSweepPath` and
 * `buildConcurrentSweepPaths` (issue #26) so both stay in lock-step on
 * ordering instead of risking two copies of the same sort drifting apart. */
function buildChildrenOf(nodes: NodeLabel[]): Map<string, NodeLabel[]> {
  const childrenOf = new Map<string, NodeLabel[]>();
  for (const n of nodes) {
    if (n.parent === null) continue;
    const siblings = childrenOf.get(n.parent);
    if (siblings) siblings.push(n);
    else childrenOf.set(n.parent, [n]);
  }
  for (const siblings of childrenOf.values()) {
    siblings.sort((a, b) => (b.metric ?? -Infinity) - (a.metric ?? -Infinity));
  }
  return childrenOf;
}

/** Depth-first sweep of `node`'s own subtree (heaviest child first,
 * backtracking to `node` between siblings so every descendant is covered),
 * appended onto `path` in place. This is the one traversal both
 * `buildFullSweepPath` (the whole tree, rooted at the graph's real root) and
 * `buildConcurrentSweepPaths` (one call per root child, each rooted at that
 * child) share — issue #26 reuses it unchanged rather than inventing a
 * second traversal for the multi-marker case. */
function sweepSubtree(node: NodeLabel, childrenOf: Map<string, NodeLabel[]>, path: NodeLabel[]): void {
  path.push(node);
  const children = childrenOf.get(node.id) ?? [];
  for (let i = 0; i < children.length; i++) {
    sweepSubtree(children[i], childrenOf, path);
    if (i < children.length - 1) path.push(node);
  }
}

/** Full depth-first sweep of the tree from its root: at each node, visits
 * children in descending-`metric` order (heaviest subtree first — same
 * intent as the old heaviest-branch-only behavior), and after each child's
 * subtree (except the last) walks back to the current node before moving on
 * to the next sibling, so every node in the tree is covered, not just one
 * branch. For a chain (every node has at most one child — traceroute hops,
 * journey demos) there's never more than one child to return from, so this
 * produces exactly the same path as following a single branch always did —
 * no behavior change for any existing chain demo. For a branching tree (a
 * dependency tree, a process tree, a Wikipedia crawl, ...) the packet now
 * sweeps the whole tree: down into a subtree, back up to the branch point,
 * down into the next one, and so on — a real Euler-tour-style traversal,
 * consecutive entries always a real parent-child edge so each step is a
 * genuine straight-line hop along the tree, not a random jump.
 * Returns `[]` if there's no root or only one node.
 *
 * Still used directly for the single-marker case (see
 * `buildConcurrentSweepPaths`'s doc comment for why a one-child root
 * degrades to producing byte-identical output to this function). */
export function buildFullSweepPath(nodes: NodeLabel[]): NodeLabel[] {
  if (nodes.length < 2) return [];
  const childrenOf = buildChildrenOf(nodes);
  const root = nodes.find((n) => n.parent === null);
  if (!root) return [];
  const path: NodeLabel[] = [];
  sweepSubtree(root, childrenOf, path);
  return path.length >= 2 ? path : [];
}

/** issue #26: multiple concurrent packets instead of one marker sweeping the
 * *entire* tree. One path per the root's direct children (already bounded by
 * the layout's own fan-out safeguard, `MAX_CHILDREN_SHOWN` = 20 — this never
 * has to re-enforce that cap itself), each independently DFS-sweeping only
 * its own subtree via the exact same `sweepSubtree` the single-marker sweep
 * uses. Every path starts with the shared root — the common "fork point" all
 * the children genuinely share — and then descends into exactly one child's
 * subtree, never crossing into a sibling's: this is what makes the markers
 * read as concurrent, independent activity (a process forking several
 * children "at once", several packets in flight) rather than N unrelated
 * partial tours.
 *
 * Degenerate cases, both handled by the general algorithm with no special
 * casing:
 * - A root with exactly one child (a plain chain, e.g. traceroute hops or
 *   `ps-tree`'s single top-level process) produces exactly one path, and
 *   that path is byte-identical to `buildFullSweepPath`'s output — there's
 *   only one child to recurse into, so no backtracking-to-root ever
 *   happens either way. A chain demo therefore renders exactly one marker,
 *   indistinguishable from the pre-#26 single-packet animation.
 * - A root with zero children (a single-node graph) returns `[]` — zero
 *   markers, not a crash.
 *
 * Returns `[]` (not a one-element array containing an empty path) for both
 * "no real root" and "root has no children", so callers can treat the
 * result as a plain list of markers to spawn with no further filtering. */
export function buildConcurrentSweepPaths(nodes: NodeLabel[]): NodeLabel[][] {
  if (nodes.length < 2) return [];
  const childrenOf = buildChildrenOf(nodes);
  const root = nodes.find((n) => n.parent === null);
  if (!root) return [];
  const rootChildren = childrenOf.get(root.id) ?? [];
  return rootChildren.map((child) => {
    const path: NodeLabel[] = [root];
    sweepSubtree(child, childrenOf, path);
    return path;
  });
}
