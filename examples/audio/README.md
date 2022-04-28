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

Then, due to my machine being x64, I want to check that it use [SIMD](https://en.wikipedia.org/wiki/Single_instruction,_multiple_data) CPU instructions. So let's look at the assembly:

```sh
RUSTFLAGS="--emit asm" cargo run --bin play-stereo --release
```

The assembly files would be found in a path like `./target/release/deps/play_stereo-79d67e2dfeee04c6.s`

Not sure how to read the resulting `.s` files? Me neither. It seems to have a bunch of `mulss` SIMD instruction but can't tell if it is the ApplyInterleaved.

Ok, let's install some helpers:

```sh
cargo install cargo-asm
```

And now show the assembly code:

```sh
cargo asm play_stereo::main
```

