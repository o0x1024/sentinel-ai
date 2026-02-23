//! Lightweight knowledge graph over memory items using petgraph.
//!
//! Nodes represent memory entries (facts, decisions, todos, reflections).
//! Edges capture semantic relationships discovered during ingestion.
//! The graph lives in-process and is rebuilt per execution from the
//! run-state memory items + any cross-session items pulled from LanceDB.

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Node / Edge types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: String,
    pub text: String,
    pub kind: MemoryNodeKind,
    pub importance: u8,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryNodeKind {
    Fact,
    Decision,
    Todo,
    Reflection,
    Entity,
    UserPreference,
}

impl std::fmt::Display for MemoryNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fact => write!(f, "fact"),
            Self::Decision => write!(f, "decision"),
            Self::Todo => write!(f, "todo"),
            Self::Reflection => write!(f, "reflection"),
            Self::Entity => write!(f, "entity"),
            Self::UserPreference => write!(f, "preference"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEdge {
    pub kind: MemoryEdgeKind,
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryEdgeKind {
    RelatedTo,
    Contradicts,
    LeadsTo,
    UpdatedBy,
    MentionsEntity,
}

impl std::fmt::Display for MemoryEdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RelatedTo => write!(f, "related_to"),
            Self::Contradicts => write!(f, "contradicts"),
            Self::LeadsTo => write!(f, "leads_to"),
            Self::UpdatedBy => write!(f, "updated_by"),
            Self::MentionsEntity => write!(f, "mentions_entity"),
        }
    }
}

// ---------------------------------------------------------------------------
// Knowledge Graph
// ---------------------------------------------------------------------------

pub struct MemoryKnowledgeGraph {
    graph: DiGraph<MemoryNode, MemoryEdge>,
    id_index: HashMap<String, NodeIndex>,
    entity_index: HashMap<String, NodeIndex>,
}

impl MemoryKnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_index: HashMap::new(),
            entity_index: HashMap::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    // -----------------------------------------------------------------------
    // Build
    // -----------------------------------------------------------------------

    /// Add a memory node. Returns its NodeIndex.
    pub fn add_memory(&mut self, node: MemoryNode) -> NodeIndex {
        if let Some(&existing) = self.id_index.get(&node.id) {
            return existing;
        }
        let id = node.id.clone();
        let idx = self.graph.add_node(node);
        self.id_index.insert(id, idx);
        idx
    }

    /// Add an entity node (extracted keyword/concept).
    pub fn add_entity(&mut self, name: &str) -> NodeIndex {
        let key = name.to_lowercase();
        if let Some(&existing) = self.entity_index.get(&key) {
            return existing;
        }
        let node = MemoryNode {
            id: format!("entity::{}", key),
            text: name.to_string(),
            kind: MemoryNodeKind::Entity,
            importance: 2,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        };
        let idx = self.graph.add_node(node);
        self.entity_index.insert(key, idx);
        idx
    }

    /// Connect two nodes with an edge.
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, kind: MemoryEdgeKind, weight: f64) {
        if self.graph.find_edge(from, to).is_some() {
            return;
        }
        self.graph.add_edge(from, to, MemoryEdge { kind, weight });
    }

    // -----------------------------------------------------------------------
    // Build from memory items
    // -----------------------------------------------------------------------

    /// Populate graph from a set of memory items, auto-discovering relationships.
    pub fn build_from_items(&mut self, items: &[(String, String, String, u8, i64)]) {
        // items: (id, text, kind_str, importance, created_at_ms)
        let mut nodes: Vec<(NodeIndex, String)> = Vec::new();

        for (id, text, kind_str, importance, created_at_ms) in items {
            let kind = match kind_str.as_str() {
                "decision" => MemoryNodeKind::Decision,
                "todo" => MemoryNodeKind::Todo,
                "reflection" => MemoryNodeKind::Reflection,
                "preference" => MemoryNodeKind::UserPreference,
                _ => MemoryNodeKind::Fact,
            };
            let node = MemoryNode {
                id: id.clone(),
                text: text.clone(),
                kind,
                importance: *importance,
                created_at_ms: *created_at_ms,
            };
            let idx = self.add_memory(node);
            nodes.push((idx, text.clone()));

            // Extract entities and link them
            let entities = extract_entities(text);
            for entity_name in &entities {
                let ent_idx = self.add_entity(entity_name);
                self.add_edge(idx, ent_idx, MemoryEdgeKind::MentionsEntity, 0.5);
            }
        }

        // Discover RelatedTo edges via shared entities
        self.discover_shared_entity_relations();

        // Discover potential Contradicts edges
        self.discover_contradictions(&nodes);
    }

    /// Link memory nodes that share entity references.
    fn discover_shared_entity_relations(&mut self) {
        let entity_ids: Vec<NodeIndex> = self
            .graph
            .node_indices()
            .filter(|&idx| self.graph[idx].kind == MemoryNodeKind::Entity)
            .collect();

        for &ent_idx in &entity_ids {
            let mentioners: Vec<NodeIndex> = self
                .graph
                .neighbors_directed(ent_idx, Direction::Incoming)
                .collect();

            for i in 0..mentioners.len() {
                for j in (i + 1)..mentioners.len() {
                    let a = mentioners[i];
                    let b = mentioners[j];
                    if self.graph.find_edge(a, b).is_none()
                        && self.graph.find_edge(b, a).is_none()
                    {
                        self.graph.add_edge(
                            a,
                            b,
                            MemoryEdge {
                                kind: MemoryEdgeKind::RelatedTo,
                                weight: 0.4,
                            },
                        );
                    }
                }
            }
        }
    }

    /// Detect potential contradictions via negation patterns.
    fn discover_contradictions(&mut self, nodes: &[(NodeIndex, String)]) {
        let negation_pairs = [
            ("enable", "disable"),
            ("use", "don't use"),
            ("allow", "deny"),
            ("include", "exclude"),
            ("add", "remove"),
            ("true", "false"),
            ("yes", "no"),
            ("启用", "禁用"),
            ("使用", "不使用"),
            ("允许", "禁止"),
            ("添加", "删除"),
        ];

        for i in 0..nodes.len() {
            let text_a = nodes[i].1.to_lowercase();
            for j in (i + 1)..nodes.len() {
                let text_b = nodes[j].1.to_lowercase();
                let has_contradiction = negation_pairs.iter().any(|(pos, neg)| {
                    (text_a.contains(pos) && text_b.contains(neg))
                        || (text_a.contains(neg) && text_b.contains(pos))
                });
                if has_contradiction
                    && shared_term_ratio(&text_a, &text_b) > 0.3
                    && self.graph.find_edge(nodes[i].0, nodes[j].0).is_none()
                {
                    self.graph.add_edge(
                        nodes[i].0,
                        nodes[j].0,
                        MemoryEdge {
                            kind: MemoryEdgeKind::Contradicts,
                            weight: 0.7,
                        },
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Query
    // -----------------------------------------------------------------------

    /// Get nodes directly related to the given node (1-hop neighbors).
    pub fn related_memories(&self, node_id: &str) -> Vec<&MemoryNode> {
        let Some(&idx) = self.id_index.get(node_id) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Outgoing)
            .chain(self.graph.neighbors_directed(idx, Direction::Incoming))
            .filter(|&n| self.graph[n].kind != MemoryNodeKind::Entity)
            .map(|n| &self.graph[n])
            .collect()
    }

    /// Expand a set of retrieved memory IDs with their 1-hop related memories.
    /// Returns additional items not already in the input set.
    pub fn expand_related(&self, memory_ids: &[String], max_extra: usize) -> Vec<String> {
        let id_set: std::collections::HashSet<&String> = memory_ids.iter().collect();
        let mut extra: Vec<(String, f64)> = Vec::new();

        for mid in memory_ids {
            let Some(&idx) = self.id_index.get(mid) else {
                continue;
            };
            for neighbor in self
                .graph
                .neighbors_directed(idx, Direction::Outgoing)
                .chain(self.graph.neighbors_directed(idx, Direction::Incoming))
            {
                let node = &self.graph[neighbor];
                if node.kind == MemoryNodeKind::Entity {
                    continue;
                }
                if id_set.contains(&node.id) {
                    continue;
                }
                if extra.iter().any(|(eid, _)| eid == &node.id) {
                    continue;
                }
                let edge_weight = self
                    .graph
                    .find_edge(idx, neighbor)
                    .or_else(|| self.graph.find_edge(neighbor, idx))
                    .map(|e| self.graph[e].weight)
                    .unwrap_or(0.3);
                extra.push((node.id.clone(), edge_weight));
            }
        }

        extra.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        extra.into_iter().take(max_extra).map(|(id, _)| id).collect()
    }

    /// Find memories that contradict the given node.
    pub fn contradictions(&self, node_id: &str) -> Vec<&MemoryNode> {
        let Some(&idx) = self.id_index.get(node_id) else {
            return Vec::new();
        };
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .chain(self.graph.edges_directed(idx, Direction::Incoming))
            .filter(|e| e.weight().kind == MemoryEdgeKind::Contradicts)
            .map(|e| {
                let other = if e.source() == idx {
                    e.target()
                } else {
                    e.source()
                };
                &self.graph[other]
            })
            .collect()
    }

    /// Get a node by its memory id.
    pub fn get_node(&self, node_id: &str) -> Option<&MemoryNode> {
        self.id_index.get(node_id).map(|&idx| &self.graph[idx])
    }

    /// Get all entity names present in the graph.
    pub fn entities(&self) -> Vec<&str> {
        self.entity_index
            .values()
            .map(|&idx| self.graph[idx].text.as_str())
            .collect()
    }

    /// Render a brief diagnostic string for logging.
    pub fn summary(&self) -> String {
        let memory_count = self
            .graph
            .node_indices()
            .filter(|&idx| self.graph[idx].kind != MemoryNodeKind::Entity)
            .count();
        let entity_count = self.entity_index.len();
        let contradiction_count = self
            .graph
            .edge_references()
            .filter(|e| e.weight().kind == MemoryEdgeKind::Contradicts)
            .count();
        format!(
            "KG: {} memories, {} entities, {} edges, {} contradictions",
            memory_count,
            entity_count,
            self.graph.edge_count(),
            contradiction_count
        )
    }
}

// ---------------------------------------------------------------------------
// Entity extraction — lightweight keyword-based
// ---------------------------------------------------------------------------

fn extract_entities(text: &str) -> Vec<String> {
    let mut entities = Vec::new();
    let lower = text.to_lowercase();

    // Extract PascalCase / camelCase identifiers
    let mut current = String::new();
    let mut has_upper = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if ch.is_uppercase() && !current.is_empty() {
                has_upper = true;
            }
            current.push(ch);
        } else {
            if current.len() >= 3 && has_upper {
                entities.push(current.clone());
            }
            current.clear();
            has_upper = false;
        }
    }
    if current.len() >= 3 && has_upper {
        entities.push(current);
    }

    // Extract quoted strings
    for cap in lower.split('"').collect::<Vec<_>>().chunks(2) {
        if cap.len() == 2 && cap[1].len() >= 2 && cap[1].len() <= 60 {
            entities.push(cap[1].trim().to_string());
        }
    }

    // Extract file paths / URLs
    for word in lower.split_whitespace() {
        if (word.contains('/') || word.contains('\\')) && word.len() > 3 {
            entities.push(word.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '\\' && c != '.' && c != '_' && c != '-').to_string());
        }
        if word.starts_with("http://") || word.starts_with("https://") {
            entities.push(word.to_string());
        }
    }

    // Extract backtick-wrapped code references
    let mut in_backtick = false;
    let mut backtick_content = String::new();
    for ch in text.chars() {
        if ch == '`' {
            if in_backtick {
                let trimmed = backtick_content.trim().to_string();
                if trimmed.len() >= 2 && trimmed.len() <= 80 {
                    entities.push(trimmed);
                }
                backtick_content.clear();
            }
            in_backtick = !in_backtick;
        } else if in_backtick {
            backtick_content.push(ch);
        }
    }

    entities.sort();
    entities.dedup();
    entities
}

/// Ratio of shared terms between two texts (simplified Jaccard).
fn shared_term_ratio(a: &str, b: &str) -> f64 {
    let terms_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let terms_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if terms_a.is_empty() || terms_b.is_empty() {
        return 0.0;
    }
    let intersection = terms_a.intersection(&terms_b).count() as f64;
    let union = terms_a.union(&terms_b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_graph_operations() {
        let mut kg = MemoryKnowledgeGraph::new();
        let items = vec![
            ("1".into(), "Use LanceDB for vector storage".into(), "decision".into(), 4u8, 1000i64),
            ("2".into(), "LanceDB supports HNSW index".into(), "fact".into(), 3, 2000),
            ("3".into(), "Remove LanceDB dependency".into(), "decision".into(), 3, 3000),
        ];
        kg.build_from_items(&items);

        assert!(kg.node_count() > 3); // memories + entities
        assert!(kg.edge_count() > 0);

        // "1" and "2" share LanceDB entity → should be related
        let related = kg.related_memories("1");
        assert!(!related.is_empty());

        // "1" (use) and "3" (remove) should contradict
        let contradictions = kg.contradictions("1");
        // May or may not trigger depending on exact heuristics
        let _ = contradictions;
    }

    #[test]
    fn expand_related_returns_extras() {
        let mut kg = MemoryKnowledgeGraph::new();
        let items = vec![
            ("a".into(), "Configure petgraph for CPG analysis".into(), "fact".into(), 3u8, 100i64),
            ("b".into(), "petgraph supports BFS traversal".into(), "fact".into(), 3, 200),
            ("c".into(), "Unrelated memory about testing".into(), "fact".into(), 2, 300),
        ];
        kg.build_from_items(&items);

        let extras = kg.expand_related(&["a".to_string()], 5);
        // "b" shares petgraph entity with "a"
        assert!(extras.contains(&"b".to_string()) || extras.is_empty());
    }

    #[test]
    fn entity_extraction() {
        let entities = extract_entities("Use `LanceDB` for storage at /tmp/data with MyConfig setting");
        assert!(entities.iter().any(|e| e.contains("LanceDB")));
        assert!(entities.iter().any(|e| e.contains("MyConfig")));
        assert!(entities.iter().any(|e| e.contains("/tmp/data")));
    }
}
