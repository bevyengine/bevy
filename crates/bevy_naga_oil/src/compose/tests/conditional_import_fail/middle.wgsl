#define_import_path middle

#ifdef USE_A
    #import a::b
#endif

fn mid_fn() -> u32 {
    return b::C;
}
