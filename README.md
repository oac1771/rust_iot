### Setup
This project uses [embassy](https://github.com/embassy-rs/embassy) as a framework for embedded systems.

#### install dependencies

install toolchain:
```
rustup toolchain install stable --component rust-src
```

install risc-v target:
```
rustup target add riscv32imc-unknown-none-elf
```

install espflash
```
cargo install espflash
```

### Reference
[embassy book](https://embassy.dev/book/index.html#_introduction)
[rust on esp book](https://docs.esp-rs.org/book/introduction.html)
[rust on esp book no_std training](https://docs.esp-rs.org/no_std-training/01_intro.html)
