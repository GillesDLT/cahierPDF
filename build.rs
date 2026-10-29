//! Compilation de l'interface Slint (`ui/main.slint`) — J2.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_build::compile("ui/main.slint")?;
    Ok(())
}
