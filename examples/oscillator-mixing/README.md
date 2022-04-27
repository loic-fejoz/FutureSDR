Oscillator Mixing
===================


An example to help work between GNU Radio Companion and FutureSDR.
It uses the ZeroMQ blocks.

## Compilation


### Linux Debian-based

```sh
sudo apt install libzmq3-dev
cd examples/oscillator-mixing
cargo build
```

### MS/Windows

#### Rust installation

First install Rust for MS/Windows as described in https://docs.microsoft.com/en-us/windows/dev-environment/rust/overview including the [CMake tool](https://docs.microsoft.com/en-us/cpp/build/cmake-projects-in-visual-studio?view=msvc-170).

At this point, both the `cargo` and `cmake` commands should be available in the prompt:

![](rust-native-tools.png)

Then better to launch VSCode for instance from within the *x64 Native Tools Command Prompt for VSXXXX* so as to make sure your environment variables are properly set.

#### Dependencies

Both [GNU Radio](https://wiki.gnuradio.org/index.php/InstallingGR) and our code will require the ZeroMQ library.

For our code, you can either install the library system-wide, or *[vendor](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html)* it by modifying the main [Cargo.toml](../../Cargo.toml) of FutureSDR as such:
```toml
zeromq = ["zmq/vendored"]
```

Depending on how you installed GNU Radio, you might need to install explicitly the ZeroMQ dependency by calling pip (or refer to [Conda packages management](https://docs.conda.io/projects/conda/en/latest/user-guide/tasks/manage-pkgs.html)):

```sh
pip install pyzmq
pip install matplotlib
```



## Running

![](grc-flowgraph.png)

On one side, open [viewer.grc](./viewer.grc) with your GNU Radio Companion and launch it.

On another side, launch the emitter with command:

```sh
cargo run
```

## Expected result

The emitter creates a 440Hz signal at a rate of 48kHz.

![](futuresdr-gnuradio.png)

And indeed the FFT display main harmonic at this 440Hz frequency.

NB: somehow there is 2 throttles (one on each sides) so we might not be in good shape but it still works eventually.
