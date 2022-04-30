FutureSDR & Audio
=================

## Introduction

FutureSDR come with some blocks interfacing the [cpal crate](https://crates.io/crates/cpal) so as to interact with sound files and audio card.

To listen the rick.mp3 file, execute:
```sh
cd examples/audio/
cargo run --bin play-file --release
```

To listen a 440Hz sound, execute:
```sh
cd examples/audio/
cargo run --bin play-file --release
```

## Performance check

After reading the article [Taking Advantage of Auto-Vectorization in Rust](https://www.nickwilcox.com/blog/autovec/), I wanted to check if the default code of FutureSDR.

So I created a [stereo version](./play_stereo.rs) of `play-file` corresponding to the same use case as in the article by first creating a generic `ApplyInterleaved` block by adapting the `Apply` block. Indeed CPAL require an interleaved stream of `f32` when doing stereo. Also see issue [#49](https://github.com/FutureSDR/FutureSDR/issues/49) around this topics. Then just pass a similar function to the newly created block to convert mono to stereo.

```rust
let mono_to_stereo = ApplyInterleaved::<f32, f32>::new(move |v: &f32, d: &mut [f32]| {
    d[0] = v * gain_l;
    d[1] = v * gain_r;
});
```

Once done, first check that it is working:

```sh
cd examples/audio/
cargo run --bin play-stereo --release
```

ok, it works.

Then, due to my machine being x64, I want to check that it use [SIMD](https://en.wikipedia.org/wiki/Single_instruction,_multiple_data) CPU instructions, in particular it should (be able to) use the `mulps` instruction. So let's look at the assembly:

NB: Thanks @bastibl to remind me to add the target-cpu.

```sh
RUSTFLAGS="-C target-cpu=native --emit asm" cargo run --bin play-stereo --release
```

The assembly files would be found in a path like `./target/release/deps/play_stereo-79d67e2dfeee04c6.s`

I can found 10 `vmulps` in it.
Yet, not sure how to read the resulting `.s` files? Me neither.
It seems to have a bunch of `mulps` SIMD instruction but can't tell if it is the ApplyInterleaved.

Ok, let's install some helpers:

```sh
cargo install cargo-asm
```

And now show the assembly code:

```sh
cargo asm play_stereo::main
```

Too complicated.
Oh! It seems we can also have code for the closure...

```sh
cargo asm play_stereo::main::{{closure}}
```

Indeed it is our code but it is not optimised with `mulps`.
Yet, is it really the final code in the release binary?!

NB: Should I enforce the inlining?

Let's try with simpler code. Do something similar [within a bench](stereo_bench.rs). Rince. Repeat...
Run bench for fun to compare effect of [Slab](https://www.futuresdr.org/blog/red-slab/):

```
$RUSTFLAGS="-C target-cpu=native"  cargo bench --package audio --bench stereo-bench --all-features
    Finished bench [optimized + debuginfo] target(s) in 0.09s
     Running unittests (/home/loic/projets/FutureSDR/target/release/deps/stereo_bench-fc5d532628ec0c42)

running 3 tests
test mono_to_stereo_1024 ... bench:   1,653,553 ns/iter (+/- 93,154)
test mono_to_stereo_2048 ... bench:   1,172,387 ns/iter (+/- 30,487)
test mono_to_stereo_4096 ... bench:   1,147,163 ns/iter (+/- 33,438)

test result: ok. 0 passed; 0 failed; 0 ignored; 3 measured; 0 filtered out; finished in 13.93s
```

Fun!
Let's check if we can find some `mulps`....

```
cargo asm stereo_bench::run_mono_to_stereo --rust
cargo asm stereo_bench::run_mono_to_stereo::{{closure}} --rust
```

Hum... :-/

Let's try the following structure as in [Nick Wilcox's Coding Blog post](https://www.nickwilcox.com/blog/autovec/):

```rust
#[repr(C)]
pub struct StereoSample {
    l: f32,
    r: f32,
}
```

```
$RUSTFLAGS="-C target-cpu=native" cargo bench --package audio --bench stereo-bench
test mono_to_stereo_1024              ... bench:   1,626,142 ns/iter (+/- 132,791)
test mono_to_stereo_1024_on_struct    ... bench:         857 ns/iter (+/- 85)
test mono_to_stereo_2048              ... bench:   1,184,636 ns/iter (+/- 40,607)
test mono_to_stereo_2048_on_struct    ... bench:       1,009 ns/iter (+/- 17)
test mono_to_stereo_4096              ... bench:   1,182,559 ns/iter (+/- 284,008)
test mono_to_stereo_4096_nm_fast      ... bench:   1,169,787 ns/iter (+/- 74,875)
test mono_to_stereo_4096_on_struct    ... bench:       1,008 ns/iter (+/- 26)
test mono_to_stereo_4096_on_struct_nm ... bench:   1,166,123 ns/iter (+/- 35,063)
```

Oh! Something different is happenning!
