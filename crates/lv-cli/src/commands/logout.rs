use crate::config::CliConfig;

pub fn run() -> anyhow::Result<()> {
    let mut cfg = CliConfig::load()?;
    cfg.token = None;
    cfg.save()?;
    println!("Logged out.");
    Ok(())
}
