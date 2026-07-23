import type { NodeLabel } from './tileset';

/** Follows the tree from its root, at each step taking the child with the
 * highest `metric` (ties keep whichever was found first), until it reaches
 * a leaf. For a chain (every node has at most one child — traceroute hops,
 * journey demos) this is trivially the complete path start-to-end, since
 * there's only one child to pick either way. For a branching tree (a
 * dependency tree, a process tree, ...) this is a real, meaningful choice —
 * "follow the heaviest branch" (e.g. the biggest subtree in a dependency
 * tree) — instead of an arbitrary "whichever happened to come first in
 * nodes.json" pick, which depended on insertion order and had no relation
 * to what the tree's own metric considers important. Still just one branch,
 * not full coverage — a multi-packet DFS sweep is a separate follow-up.
 * Returns `[]` if there's no root or only one node. */
export function buildPrimaryPath(nodes: NodeLabel[]): NodeLabel[] {
  if (nodes.length < 2) return [];

  const childrenOf = new Map<string, NodeLabel[]>();
  for (const n of nodes) {
    if (n.parent === null) continue;
    const siblings = childrenOf.get(n.parent);
    if (siblings) siblings.push(n);
    else childrenOf.set(n.parent, [n]);
  }

  const root = nodes.find((n) => n.parent === null);
  if (!root) return [];

  const path: NodeLabel[] = [root];
  const visited = new Set<string>([root.id]);
  let current = root;
  for (;;) {
    const children = childrenOf.get(current.id);
    if (!children || children.length === 0) break;
    let next = children[0];
    for (const child of children) {
      if ((child.metric ?? -Infinity) > (next.metric ?? -Infinity)) next = child;
    }
    if (visited.has(next.id)) break;
    path.push(next);
    visited.add(next.id);
    current = next;
  }
  return path.length >= 2 ? path : [];
}
