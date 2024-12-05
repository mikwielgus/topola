# Fuzzer for `specctra-core`

This fuzzer uses [`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz/tutorial.html).
It requires nightly rust to run, and can e.g. be invoked via `cargo fuzz run fuzz_target_1`.

Initialize the corpus via e.g.:
```
mkdir -p corpus/fuzz_target_1
cp -t corpus/fuzz_target_1 $(find ../../../tests | grep \\.dsn)
```
before invoking the fuzzer in order to provide the fuzzing with information how
input should normally look like.
