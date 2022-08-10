use naga_oil::compose::{ShaderLanguage, Composer, ComposerError};

fn main() -> Result<(), ComposerError> {
    let mut composer = Composer::default();

    composer.add_composable_module(
        include_str!("nested_const/consts.wgsl").to_string(),
        ShaderLanguage::Wgsl,
    )?;
    composer.add_composable_module(
        include_str!("nested_const/a.wgsl").to_string(),
        ShaderLanguage::Wgsl,
    )?;
    composer.add_composable_module(
        include_str!("nested_const/b.wgsl").to_string(),
        ShaderLanguage::Wgsl,
    )?;
    let module = composer.make_naga_module(
        include_str!("nested_const/top.wgsl").to_string(),
        ShaderLanguage::Wgsl,
        &[],
    )?;

    let info = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default()).validate(&module).unwrap();
    let wgsl = naga::back::wgsl::write_string(&module, &info, naga::back::wgsl::WriterFlags::EXPLICIT_TYPES).unwrap();

    println!("wgsl: \n {}", wgsl);    

    Ok(())
}