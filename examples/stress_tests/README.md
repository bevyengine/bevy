# Stress tests

These examples are used to stress test Bevy's performance in various ways. These
should be run with the "stress-test" profile to accurately represent performance
in production, otherwise they will run in cargo's default "dev" profile which is
very slow.

## Example Command

```bash
cargo run --profile stress-test --example <EXAMPLE>
```
