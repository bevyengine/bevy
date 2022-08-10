use naga_oil::compose::{Composer, ComposerError, ShaderLanguage};

fn main() -> Result<(), ComposerError> {
    let mut composer = Composer::default();

    composer.add_composable_module(
        include_str!("simple/inc.wgsl").to_string(),
        ShaderLanguage::Wgsl,
    )?;
    let module = composer.make_naga_module(
        include_str!("simple/top.wgsl").to_string(),
        ShaderLanguage::Wgsl,
        &[],
    )?;

    println!("module: {:?}", module);

    Ok(())
}
