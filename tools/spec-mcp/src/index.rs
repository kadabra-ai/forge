use anyhow::Result;
use std::collections::HashMap;

use crate::bnf::{parse_bnf, Production};
use crate::clause_tree::{extract_clause_tree, ClauseNode};

pub struct SpecIndex {
    src: String,
    clauses: Vec<ClauseNode>,
    by_id: HashMap<String, usize>,
    productions: Vec<Production>,
    productions_by_clause: HashMap<String, Vec<usize>>,
}

impl SpecIndex {
    pub fn list_clauses(&self) -> &[ClauseNode] {
        &self.clauses
    }

    pub fn clause(&self, id: &str) -> Option<&ClauseNode> {
        self.by_id.get(id).map(|&i| &self.clauses[i])
    }

    pub fn productions(&self) -> &[Production] {
        &self.productions
    }

    pub fn productions_for_clause(&self, clause_id: &str) -> Vec<&Production> {
        self.productions_by_clause
            .get(clause_id)
            .map(|idxs| idxs.iter().map(|&i| &self.productions[i]).collect())
            .unwrap_or_default()
    }

    pub fn body_for_clause(&self, clause_id: &str) -> Option<&str> {
        let clause = self.clause(clause_id)?;
        Some(&self.src[clause.body_start..clause.body_end])
    }
}

pub fn build_index(md: &str, bnf_src: &str) -> Result<SpecIndex> {
    let clauses = extract_clause_tree(md)?;
    let by_id: HashMap<String, usize> = clauses
        .iter()
        .enumerate()
        .map(|(i, c)| (c.id.clone(), i))
        .collect();

    let productions = parse_bnf(bnf_src)?;
    let mut productions_by_clause: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, p) in productions.iter().enumerate() {
        productions_by_clause
            .entry(p.clause_id.clone())
            .or_default()
            .push(i);
    }

    Ok(SpecIndex {
        src: md.to_string(),
        clauses,
        by_id,
        productions,
        productions_by_clause,
    })
}