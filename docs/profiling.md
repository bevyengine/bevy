# Profiling

* Compile Times: append ```-Ztimings``` to cargo builds
* Runtime Flame Graph:
  * Flat-ish: ```RUSTFLAGS='-C force-frame-pointers=y' cargo flamegraph --example EXAMPLE_NAME```
  * Graph: ```RUSTFLAGS='-C force-frame-pointers=y' cargo flamegraph -c "record -g" --example EXAMPLE_NAME```
  * built on top of perf, no instrumentation required
* Runtime Instrumentation:
  * [thread_profiler](https://github.com/glennw/thread_profiler)
