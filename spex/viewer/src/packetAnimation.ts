import type { NodeLabel } from './tileset';

/** Follows the tree from its root, always taking the first child, until it
 * reaches a leaf. For a chain (every node has at most one child — traceroute
 * hops, journey demos) this is the complete path start-to-end. For a
 * branching tree it's an honest partial path down one branch rather than a
 * nonsense jump across siblings — full branch coverage is a follow-up, not
 * solved here. Returns `[]` if there's no root or only one node. */
export function buildPrimaryPath(nodes: NodeLabel[]): NodeLabel[] {
  if (nodes.length < 2) return [];

  const byId = new Map<string, NodeLabel>();
  const firstChildOf = new Map<string, NodeLabel>();
  for (const n of nodes) byId.set(n.id, n);
  for (const n of nodes) {
    if (n.parent !== null && !firstChildOf.has(n.parent)) {
      firstChildOf.set(n.parent, n);
    }
  }

  const root = nodes.find((n) => n.parent === null);
  if (!root) return [];

  const path: NodeLabel[] = [root];
  const visited = new Set<string>([root.id]);
  let current = root;
  for (;;) {
    const next = firstChildOf.get(current.id);
    if (!next || visited.has(next.id)) break;
    path.push(next);
    visited.add(next.id);
    current = next;
  }
  return path.length >= 2 ? path : [];
}
