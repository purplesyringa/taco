# taco

A one-day experiment in domain-specific compression.

The program can be built using `cargo +nightly build --release` and invoked using `./target/release/taco <file1> <file2> <...>`. If multiple filenames are passed, the files are combined into a single packet: filenames and everything else but file boundaries is lost.

Taco requires input files to be UTF-8-encoded and achieves good compression ratios on typical competitive programming test files. On average, compressed file size is about 30% less than that of zstd with default parameters and 15% less than that of LZMA with default parameters.

Compression time is worse than terrible (I'm not even sure if it's exponential), but it seems to work relatively fast in practice, if 2 minute long compression of a single test is anything to go by.
