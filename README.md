# androidy-log

![Rust](https://github.com/DoumanAsh/androidy-log/workflows/Rust/badge.svg?branch=master)
[![Crates.io](https://img.shields.io/crates/v/androidy-log.svg)](https://crates.io/crates/androidy-log)
[![Documentation](https://docs.rs/androidy-log/badge.svg)](https://docs.rs/crate/androidy-log/)

Minimal wrapper over android logging facilities.

## Features:

- `std` - Enables `std::io::Write` implementation.

## Usage

```rust
use androidy_log::{LogPriority, Writer};

use core::fmt::Write;

let mut writer = Writer::new("MyTag", LogPriority::INFO);
let _ = write!(writer, "Hellow World!");
drop(writer) //or writer.flush();
```
