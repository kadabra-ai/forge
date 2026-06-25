use anyhow::Result;
use std::path::PathBuf;

use rmcp::handler::server::wrapper::Json;
use rmcp::tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::implementation::find_implementation;
use crate::index::{build_index, SpecIndex};

pub struct SpecServer {
    index: SpecIndex,
    crates_dir: PathBuf,
}

impl SpecServer {
    pub fn new(md: String, bnf_src: String, crates_dir: PathBuf) -> Result<Self> {
        let index = build_index(&md, &bnf_src)?;
        Ok(Self { index, crates_dir })
    }

    pub fn list_sections_impl(&self) -> Vec<ClauseSummary> {
        self.index
            .list_clauses()
            .iter()
            .map(|c| ClauseSummary {
                id: c.id.clone(),
                title: c.title.clone(),
                anchor: c.anchor.clone(),
            })
            .collect()
    }

    pub fn get_section_impl(&self, clause_id: &str) -> Option<SectionResult> {
        let clause = self.index.clause(clause_id)?;
        let productions = self.index.productions_for_clause(clause_id);
        let body = self
            .index
            .body_for_clause(clause_id)
            .unwrap_or_default()
            .trim()
            .to_string();
        Some(SectionResult {
            id: clause.id.clone(),
            title: clause.title.clone(),
            anchor: clause.anchor.clone(),
            line: clause.line,
            body_md: body,
            productions: productions.iter().map(|p| p.name.clone()).collect(),
        })
    }

    pub fn find_implementation_impl(&self, clause_id: &str) -> ImplResult {
        let productions: Vec<&str> = self
            .index
            .productions_for_clause(clause_id)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        match find_implementation(clause_id, &productions, &self.crates_dir) {
            Ok(r) => ImplResult {
                status: format!("{:?}", r.status),
                entry_points: r.entry_points,
            },
            Err(e) => ImplResult {
                status: format!("error: {e}"),
                entry_points: vec![],
            },
        }
    }

    pub fn search_sections_impl(&self, query: &str) -> Vec<ClauseSummary> {
        let q = query.to_lowercase();
        self.index
            .list_clauses()
            .iter()
            .filter(|c| c.id.contains(&q) || c.title.to_lowercase().contains(&q))
            .map(|c| ClauseSummary {
                id: c.id.clone(),
                title: c.title.clone(),
                anchor: c.anchor.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClauseSummary {
    pub id: String,
    pub title: String,
    pub anchor: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct SectionQuery {
    pub clause_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClauseListResult {
    pub clauses: Vec<ClauseSummary>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct SectionResult {
    pub id: String,
    pub title: String,
    pub anchor: String,
    pub line: usize,
    pub body_md: String,
    pub productions: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSectionResult {
    pub section: Option<SectionResult>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplQuery {
    pub clause_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImplResult {
    pub status: String,
    pub entry_points: Vec<String>,
}

#[rmcp::tool_router]
impl SpecServer {
    #[tool(name = "list_sections", description = "List the KerML spec clause tree")]
    fn list_sections(&self) -> Json<ClauseListResult> {
        Json(ClauseListResult {
            clauses: self.list_sections_impl(),
        })
    }

    #[tool(name = "get_section", description = "Get a KerML spec section by clause id")]
    fn get_section(
        &self,
        rmcp::handler::server::wrapper::Parameters(SectionQuery { clause_id }): rmcp::handler::server::wrapper::Parameters<SectionQuery>,
    ) -> Json<GetSectionResult> {
        Json(GetSectionResult {
            section: self.get_section_impl(&clause_id),
        })
    }

    #[tool(name = "search_sections", description = "Search KerML spec sections by id or title")]
    fn search_sections(
        &self,
        rmcp::handler::server::wrapper::Parameters(SearchQuery { query }): rmcp::handler::server::wrapper::Parameters<SearchQuery>,
    ) -> Json<ClauseListResult> {
        Json(ClauseListResult {
            clauses: self.search_sections_impl(&query),
        })
    }

    #[tool(name = "find_implementation", description = "Find compiler entry points implementing a clause")]
    fn find_implementation(
        &self,
        rmcp::handler::server::wrapper::Parameters(ImplQuery { clause_id }): rmcp::handler::server::wrapper::Parameters<ImplQuery>,
    ) -> Json<ImplResult> {
        Json(self.find_implementation_impl(&clause_id))
    }
}

#[rmcp::tool_handler]
impl rmcp::ServerHandler for SpecServer {}