#define_import_path wgsl_parse_err

const VAL: u32 = 1u;

fn all_ok() -> f32 {
    let x = 1.0;
    var y = sqrt(x);
    y += 1.0;
    return y;
}

fn woops() -> f32 {
    let x = 1.0;
    var y = sqrt(x);
    y += 1.0;
    return zdd; 
}