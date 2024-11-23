#define_import_path test_module

var<workgroup> atom: atomic<u32>;

fn entry_point() -> f32 {
    atomicStore(&atom, 1u);          // atom = 1
    var y = atomicLoad(&atom);       // y = 1, atom = 1
    y += atomicAdd(&atom, 2u);       // y = 2, atom = 3
    y += atomicSub(&atom, 1u);       // y = 5, atom = 2
    y += atomicMax(&atom, 5u);       // y = 7, atom = 5
    y += atomicMin(&atom, 4u);       // y = 12, atom = 4
    y += atomicExchange(&atom, y);  // y = 16, atom = 12
    let exchange = atomicCompareExchangeWeak(&atom, 12u, 0u);
    if exchange.exchanged {
        y += exchange.old_value;    // y = 28, atom = 0
    }

    return f32(y); // 28.0
}