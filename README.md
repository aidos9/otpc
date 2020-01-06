# OTPC
A One-Time Password Client, using the TOTP algorithm.

## Table of contents
* [Usage](#Usage)
* [Installing](#Installing)
* [Building](#Building)

## Usage
In OTPC each item is tracked through its label, and can be created, viewed, or edited from the command line but it may be easier to use the interactive mode which displays a UI using tui(curses).
To see the available options run:
```
./otpc --help
```

## Installing
The latest version may be installed or updated using:
```
cargo install --git https://github.com/aidos9/otpc.git
```

## Building

#### Features
```
"interactive" - Enable the interactive option. Enabled by default
```

#### Compiling

OTPC is written in Rust and uses cargo for dependency management and compilation. To build and install run:
```
cargo build --release
```

To build without interactive mode:
```
cargo build --release --no-default-features
```
