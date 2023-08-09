SSB
===

Examples of an SSB modulator (`receive`) and demodulator (`transmit`).

## Usage

### Receive

```
cargo run --release --bin receive
```

By default, this reads the file `ssb_lsb_256k_complex2.dat` and outputs the audio to your default audio device.

You can download that file from https://www.csun.edu/~skatz/katzpage/sdr_project/sdr/ssb_lsb_256k_complex2.dat.zip or by running `make ssb_lsb_256k_complex2.dat`

![](flowgraph-2022-07-28-124646.png)

### Transmit

```
cargo run --release --bin receive INPUT OUTPUT
```

* `INPUT` must be an audio file with only one chanel (mono).
* `OUTPUT` will be a wave file containing the complex IQ data generated.

You can view/listen to it using an software define radio application like [SDR++](https://www.sdrpp.org/).

## Architecture

Goals is to achieve SSB decoding as in:
* https://wiki.gnuradio.org/index.php/Simulation_example:_Single_Sideband_transceiver
* http://www.csun.edu/~skatz/katzpage/sdr_project/sdr/grc_tutorial4.pdf

So really have same result as this [GNURadio flowgraph](./ssb-decoder.grc).

## Roundtrip example

Lets pick a WAV file, eg `test-123-test.wav`.

One can first generate IQ file (complex 32 file format) with following command line:

```sh
cargo run --bin transmit -- -m USB test-123-test.wav test-123-test-ssb
```

It results in two files actually: one `.wav` file containing IQ in int16 format, and one `.c32` file containing IQ but in interleaved complex f32 format.

So now, one can listen back to the result with:

```sh
cargo run --bin receive -- -f test-123-test-ssb.c32 --center-freq 50950 --file-level 3.0
```

The result may not sound great depending on few factors. First adjust the center of frequency, then the signal level if needed.
You should be able to hear your initial message back.