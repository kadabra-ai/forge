use anyhow::Result;
use rmcp::ServiceExt;
use spec_mcp::tools::SpecServer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let md_path = repo_root
        .join("docs/spec/1-Kernel_Modeling_Language/1-Kernel_Modeling_Language.md");
    let bnf_path = repo_root.join("vendor/SysML-v2-Release/bnf/KerML-textual-bnf.kebnf");
    let crates_dir = repo_root.join("crates");

    let md = std::fs::read_to_string(&md_path)?;
    let bnf = std::fs::read_to_string(&bnf_path)?;
    let server = SpecServer::new(md, bnf, crates_dir)?;

    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let running = server.serve(transport).await?;
    running.waiting().await?;
    Ok(())
}