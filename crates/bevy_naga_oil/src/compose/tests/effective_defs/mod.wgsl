#define_import_path mod

#ifdef DEF_ONE
const a: u32 = 1u;
#else 
const a: u32 = 0u;
#endif

#ifndef DEF_TWO
const b: u32 = 0u;
#else
const b: u32 = 2u;
#endif

#if DEF_THREE == true
const c: u32 = 4u;
#else
const c: u32 = 0u;
#endif