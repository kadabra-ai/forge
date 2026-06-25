use spec_mcp::tools::SpecServer;

fn build_test_server() -> SpecServer {
    let md = "## <span id=\"page-51-0\"></span>**7.3.2 Types**\n\n## <span id=\"page-52-0\"></span>**7.3.2.3 Specialization**\n";
    let bnf = "// Clause 8.2.4.1.2 Specialization\n\nSpecialization =\n    'specializes'\n\n// Clause 8.2.4.3.1 Features\n\nFeatureDeclaration : Feature =\n    'feature'\n";
    let crates_dir = std::path::PathBuf::from("../../crates");
    SpecServer::new(md.to_string(), bnf.to_string(), crates_dir).unwrap()
}

#[test]
fn constructs_server_from_synthetic_spec() {
    let _ = build_test_server();
}

#[test]
fn get_section_returns_title_and_anchor() {
    let server = build_test_server();
    let result = server.get_section_impl("7.3.2").unwrap();
    assert_eq!(result.title, "Types");
    assert_eq!(result.anchor, "page-51-0");
    assert!(result.productions.is_empty());
}

#[test]
fn get_section_returns_none_for_missing_clause() {
    let server = build_test_server();
    assert!(server.get_section_impl("99.99").is_none());
}

#[test]
fn list_sections_returns_all_clauses() {
    let server = build_test_server();
    let clauses = server.list_sections_impl();
    assert_eq!(clauses.len(), 2);
    assert!(clauses.iter().any(|c| c.id == "7.3.2"));
    assert!(clauses.iter().any(|c| c.id == "7.3.2.3"));
}

#[test]
fn find_implementation_returns_status() {
    let server = build_test_server();
    let result = server.find_implementation_impl("8.2.4.3.1");
    assert_eq!(result.status, "Implemented");
    assert!(result
        .entry_points
        .iter()
        .any(|e| e.contains("parse_feature_decl")));
}

#[test]
fn get_section_includes_productions() {
    let md = "## <span id=\"page-51-0\"></span>**8.2.4.1.2 Specialization**\n";
    let bnf = "// Clause 8.2.4.1.2 Specialization\n\nSpecialization =\n    'specializes'\n";
    let crates_dir = std::path::PathBuf::from("../../crates");
    let server = SpecServer::new(md.to_string(), bnf.to_string(), crates_dir).unwrap();
    let result = server.get_section_impl("8.2.4.1.2").unwrap();
    assert!(result.productions.contains(&"Specialization".to_string()));
}