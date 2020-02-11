# Profiling

* Compile Times: append ```-Ztimings``` to cargo builds
* Runtime Flame Graph: ```cargo flamegraph --example EXAMPLE_NAME```
    * built on top of perf, no instrumentation required
* Runtime Instrumentation:
    * https://github.com/glennw/thread_profiler