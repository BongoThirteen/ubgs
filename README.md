An Unnamed Block Game Server.

This is a Minecraft-compatible server built using the [Valence](https://github.com/valence-rs/valence) framework.
It is designed to be similar in functionality to the official server, yet significantly more performant.
Due to the excellent plugin architecture of [Bevy ECS](https://bevyengine.org), it is also highly customizable.

Features such as entity physics, redstone, terrain generation, and world file loading and saving, are under development.
Ultimately, this project aims to provide a complete replacement for the official servers written in Rust and permissively licensed.
However, at the moment there are many bugs and incomplete features.

# Getting Started

Currently no application binaries are provided, so just create a new Rust project and import this repository.
```bash
cargo new blockgame
cd blockgame
```
Edit `Cargo.toml` and add this line under `[dependencies]`:
```toml
ubgs = { git = "https://github.com/BongoThirteen/ubgs.git" }
```
Then, write this in your `src/main.rs`:
```rust
use ubgs::Vanilla;
use ubgs::prelude::*;

fn main() {
  App::new()
    .add_plugins(Vanilla)
    .run();
}
```
Finally, run the code with `cargo run` and connect with your Minecraft-compatible client of choice to `localhost`.

# Licensing

Like most Rust software, this project is dually licensed under
* the [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0) or
* the [MIT license](http://opensource.org/licenses/MIT)

at your option.
