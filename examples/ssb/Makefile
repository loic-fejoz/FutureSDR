run: ssb_lsb_256k_complex2.dat
	cargo run --release --bin receive

ssb_lsb_256k_complex2.dat.zip:
	wget https://www.csun.edu/~skatz/katzpage/sdr_project/sdr/ssb_lsb_256k_complex2.dat.zip

ssb_lsb_256k_complex2.dat: ssb_lsb_256k_complex2.dat.zip
	unzip -DD $<

test-123-test-ssb.c32: test-123-test-ssb.wav
	cargo run --bin transmit -- -m USB test-123-test.wav test-123-test-ssb

test-123-test-ssb-listen: test-123-test-ssb.c32
	cargo run --bin receive -- -f test-123-test-ssb.c32 --center-freq 50950 --file-level 3.0

.PHONY: clean
clean:
	rm -f ssb_lsb_256k_complex2.dat ssb_lsb_256k_complex2.dat.zip
