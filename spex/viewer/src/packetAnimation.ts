import type { NodeLabel } from './tileset';

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
 * Returns `[]` if there's no root or only one node. */
export function buildFullSweepPath(nodes: NodeLabel[]): NodeLabel[] {
  if (nodes.length < 2) return [];

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

  const root = nodes.find((n) => n.parent === null);
  if (!root) return [];

  const path: NodeLabel[] = [];
  function visit(node: NodeLabel) {
    path.push(node);
    const children = childrenOf.get(node.id) ?? [];
    for (let i = 0; i < children.length; i++) {
      visit(children[i]);
      if (i < children.length - 1) path.push(node);
    }
  }
  visit(root);
  return path.length >= 2 ? path : [];
}
