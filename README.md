# OTPC
A One-Time Password Client, using the TOTP algorithm.

## Table of contents
* [Usage](#Usage)
* [Building](#Building)

## Usage
In OTPC each item is tracked through its label, and can be created, viewed, or edited from the command line but it may be easier to use the interactive mode which displays a UI using tui(curses).
To see the available options run:
```
./otpc --help
```

## Building

#### Features
```
"interactive" - Enable the interactive option. Enabled by default
```

#### Compiling and installation

OTPC is written in Rust and uses cargo for dependency management and compilation. To build and install run:
```
cargo build --release
cargo install
```

To build without interactive mode:
```
cargo build --release --no-default-features
cargo install
```
