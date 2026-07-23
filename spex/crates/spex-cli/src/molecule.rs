use anyhow::{bail, Result};
use serde_json::{Map, Value};
use spex_graph::{Graph, GraphNode};
use std::collections::HashMap;

/// A handful of real, well-known molecules (their actual canonical SMILES
/// strings) so `spex molecule` has something to point at without requiring
/// the caller to already know SMILES syntax.
pub const KNOWN_MOLECULES: &[(&str, &str)] = &[
    ("ethanol", "CCO"),
    ("benzene", "c1ccccc1"),
    ("aspirin", "CC(=O)OC1=CC=CC=C1C(=O)O"),
    ("caffeine", "CN1C=NC2=C1C(=O)N(C(=O)N2C)C"),
];

fn atomic_number(symbol: &str) -> u32 {
    match symbol {
        "H" => 1,
        "C" | "c" => 6,
        "N" | "n" => 7,
        "O" | "o" => 8,
        "F" => 9,
        "P" | "p" => 15,
        "S" | "s" => 16,
        "Cl" => 17,
        "Br" => 35,
        "I" => 53,
        _ => 0,
    }
}

struct Atom {
    symbol: String,
    aromatic: bool,
}

/// Real SMILES ring-closure bonds (digit pairs, e.g. the two `1`s in
/// `c1ccccc1`) don't fit `Graph`'s tree-only model — a benzene ring closing
/// back on itself would give one atom two parents. Rather than dropping the
/// bond, or forcing a bigger cycle-support rewrite of `Graph` (see
/// TODOs.md), the closing bond is kept as metadata on both endpoint atoms
/// (`ring_bond_to`) instead of a second tree edge — the real ring
/// connectivity is visible on hover even though the layout only draws the
/// spanning-tree edges. Same "real forcing case for the DAG/cycle
/// limitation" the backlog flagged this adapter for.
struct RingClosure {
    atom_index: usize,
    bond_order: u32,
}

/// Parses a (deliberately simplified) subset of real SMILES: the organic
/// subset's bare atoms (`C`, `N`, `O`, `F`, `P`, `S`, `Cl`, `Br`, `I`),
/// lowercase aromatic atoms (`c`, `n`, `o`, `p`, `s`), single/double/triple
/// bonds (`-`/`=`/`#`, single is also the default with no symbol),
/// branches (`(...)`), and ring-closure digits. Bracket atoms
/// (`[nH]`, charges, isotopes, stereochemistry `@`/`@@`) are accepted but
/// their extra detail beyond the element symbol is not modeled — this is
/// a graph-shape demo, not a cheminformatics engine.
pub fn parse_smiles(smiles: &str) -> Result<Graph> {
    let chars: Vec<char> = smiles.chars().collect();
    let mut i = 0;
    let mut atoms: Vec<Atom> = Vec::new();
    // (parent_atom_index, bond_order) per atom, None for the very first atom (the root)
    let mut parent_of: Vec<Option<(usize, u32)>> = Vec::new();
    let mut ring_bonds: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut ring_closures: Vec<(usize, RingClosure)> = Vec::new();

    let mut branch_stack: Vec<usize> = Vec::new();
    let mut current: Option<usize> = None;
    let mut pending_bond: u32 = 1;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '(' => {
                if let Some(cur) = current {
                    branch_stack.push(cur);
                }
                i += 1;
            }
            ')' => {
                current = branch_stack.pop();
                i += 1;
            }
            '-' => {
                pending_bond = 1;
                i += 1;
            }
            '=' => {
                pending_bond = 2;
                i += 1;
            }
            '#' => {
                pending_bond = 3;
                i += 1;
            }
            ':' => {
                pending_bond = 1;
                i += 1;
            }
            '0'..='9' => {
                let ring_id = c.to_digit(10).unwrap();
                let atom_idx = current.ok_or_else(|| {
                    anyhow::anyhow!("ring-closure digit '{}' appears before any atom", c)
                })?;
                let entry = ring_bonds.entry(ring_id).or_default();
                entry.push(atom_idx);
                if entry.len() == 2 {
                    let a = entry[0];
                    let b = entry[1];
                    ring_closures.push((a, RingClosure { atom_index: b, bond_order: pending_bond }));
                    ring_closures.push((b, RingClosure { atom_index: a, bond_order: pending_bond }));
                    ring_bonds.remove(&ring_id);
                }
                pending_bond = 1;
                i += 1;
            }
            '[' => {
                let close = chars[i..]
                    .iter()
                    .position(|&ch| ch == ']')
                    .ok_or_else(|| anyhow::anyhow!("unterminated '[' bracket atom"))?;
                let inside: String = chars[i + 1..i + close].iter().collect();
                let symbol: String = inside
                    .chars()
                    .take_while(|ch| ch.is_ascii_alphabetic())
                    .collect();
                if symbol.is_empty() {
                    bail!("bracket atom '[{inside}]' has no element symbol");
                }
                let aromatic = symbol.chars().next().unwrap().is_lowercase();
                let atom_index = atoms.len();
                atoms.push(Atom { symbol, aromatic });
                parent_of.push(current.map(|p| (p, pending_bond)));
                pending_bond = 1;
                current = Some(atom_index);
                i += close + 1;
            }
            'A'..='Z' | 'a'..='z' => {
                // Two-letter elements first (Cl, Br), else a single-letter organic-subset atom.
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                let (symbol, consumed) = if two == "Cl" || two == "Br" {
                    (two, 2)
                } else {
                    (c.to_string(), 1)
                };
                let aromatic = symbol.chars().next().unwrap().is_lowercase();
                let atom_index = atoms.len();
                atoms.push(Atom { symbol, aromatic });
                parent_of.push(current.map(|p| (p, pending_bond)));
                pending_bond = 1;
                current = Some(atom_index);
                i += consumed;
            }
            _ => {
                // Whitespace or unsupported syntax (stereo bonds '/', '\\', etc.) — skip.
                i += 1;
            }
        }
    }

    if atoms.is_empty() {
        bail!("no atoms parsed from SMILES string {smiles:?}");
    }
    if !ring_bonds.is_empty() {
        bail!("unclosed ring-bond digit(s) in SMILES string {smiles:?}");
    }

    let mut ring_meta: HashMap<usize, Vec<String>> = HashMap::new();
    for (atom_idx, closure) in &ring_closures {
        ring_meta
            .entry(*atom_idx)
            .or_default()
            .push(format!("a{} (bond order {})", closure.atom_index, closure.bond_order));
    }

    let nodes = atoms
        .iter()
        .enumerate()
        .map(|(idx, atom)| {
            let mut metadata = Map::new();
            metadata.insert("element".to_string(), Value::from(atom.symbol.clone()));
            metadata.insert("aromatic".to_string(), Value::from(atom.aromatic));
            if let Some(rings) = ring_meta.get(&idx) {
                metadata.insert(
                    "ring_bond_to".to_string(),
                    Value::from(rings.join(", ")),
                );
            }
            if let Some((_, bond_order)) = parent_of[idx] {
                metadata.insert("bond_order".to_string(), Value::from(bond_order));
            }
            GraphNode {
                id: format!("a{idx}"),
                label: atom.symbol.clone(),
                parent: parent_of[idx].map(|(p, _)| format!("a{p}")),
                metric: Some(atomic_number(&atom.symbol) as f64),
                metadata,
            }
        })
        .collect();

    Ok(Graph {
        title: Some(format!("molecule: {smiles}")),
        metric_label: Some("atomic number".to_string()),
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ethanol_is_a_simple_chain() {
        let g = parse_smiles("CCO").unwrap();
        assert_eq!(g.nodes.len(), 3);
        assert_eq!(g.nodes[0].label, "C");
        assert_eq!(g.nodes[1].label, "C");
        assert_eq!(g.nodes[2].label, "O");
        assert_eq!(g.nodes[0].parent, None);
        assert_eq!(g.nodes[1].parent, Some("a0".to_string()));
        assert_eq!(g.nodes[2].parent, Some("a1".to_string()));
    }

    #[test]
    fn benzene_ring_closure_is_metadata_not_a_second_parent() {
        let g = parse_smiles("c1ccccc1").unwrap();
        assert_eq!(g.nodes.len(), 6);
        assert!(g.nodes.iter().all(|n| n.metadata["aromatic"] == true));
        // Every atom has exactly one Graph parent (or none, for the root) —
        // the ring-closing 6th->1st bond must NOT create a second parent.
        assert_eq!(g.nodes[0].parent, None);
        for n in &g.nodes[1..] {
            assert!(n.parent.is_some());
        }
        // The real ring bond (atom 0 <-> atom 5) is preserved as metadata on both ends.
        assert!(g.nodes[0].metadata.get("ring_bond_to").is_some());
        assert!(g.nodes[5].metadata.get("ring_bond_to").is_some());
    }

    #[test]
    fn branches_reattach_to_the_correct_parent() {
        // Acetic acid: CC(=O)O -> C0-C1, C1=O2 (branch), C1-O3 (after branch closes)
        let g = parse_smiles("CC(=O)O").unwrap();
        assert_eq!(g.nodes.len(), 4);
        assert_eq!(g.nodes[1].parent, Some("a0".to_string()));
        assert_eq!(g.nodes[2].parent, Some("a1".to_string()));
        assert_eq!(g.nodes[3].parent, Some("a1".to_string()));
        assert_eq!(g.nodes[2].metadata["bond_order"], Value::from(2));
    }

    #[test]
    fn all_known_molecules_parse() {
        for (name, smiles) in KNOWN_MOLECULES {
            let g = parse_smiles(smiles)
                .unwrap_or_else(|e| panic!("failed to parse {name} ({smiles}): {e}"));
            assert!(!g.nodes.is_empty(), "{name} produced no atoms");
        }
    }
}
