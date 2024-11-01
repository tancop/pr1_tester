# Tester PR1

Works just like [FedorViest's tester](https://github.com/FedorViest/opp_prpr2024/tree/main/Tester) but faster and supports Windows.

## Install

- Download the latest release for your OS
- Unpack in your work folder
- Set the compile and run commands in .env
- Run the tester from a terminal window (`./tester` on Linux, `.\tester.exe` on Windows)

## Build

- Install [Rust](https://www.rust-lang.org/tools/install)
- Build the project:
  - `cargo run` to build and run
  - `cargo build --release` for an optimized release build

## `.env` examples

### Clang on Linux

```bash
TESTER_COMPILE_COMMAND="clang -O1 main.c"
TESTER_RUN_COMMAND="./a.out"
TESTER_INPUT_FILES="data.txt;string.txt;parse.txt"
```

### GCC on Windows

```bash
TESTER_COMPILE_COMMAND="gcc -O1 main.c"
TESTER_RUN_COMMAND=".\a.exe"
TESTER_INPUT_FILES="data.txt;string.txt;parse.txt"
```
