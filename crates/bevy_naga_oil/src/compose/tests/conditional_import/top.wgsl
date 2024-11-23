#ifdef USE_A
    #import a C
#else
    #import b C
#endif

fn main() -> u32 {
    return C;
}