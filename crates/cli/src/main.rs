fn main() -> anyhow::Result<()> {
    println!("rombro {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
