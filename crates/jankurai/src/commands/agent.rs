use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AgentVerifyArgs {
    pub repo: PathBuf,
}

pub fn verify(args: AgentVerifyArgs) -> Result<()> {
    let failures = crate::init::adapters::verify_adapters(&args.repo)?;
    if failures.is_empty() {
        println!("agent adapters verified");
        return Ok(());
    }
    for failure in &failures {
        println!("high: {} - {}", failure.path, failure.problem);
    }
    anyhow::bail!("agent verification found {} adapter issues", failures.len());
}
