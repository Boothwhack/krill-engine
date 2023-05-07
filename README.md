# Krill game engine
This project is built with technologies that support compiling to a multitude of platforms. The Rust compiler has 
built-in support for compiling to WebAssembly, which allows for near-native performance on the web.

## Demo
A demo is available [here](https://boothwhack.github.io/krill-engine/) in the form of a recreation of a classic arcade 
game.

The demo is built as a binary crate located in the [meteors](meteors) directory. This crate is written using primarily platform-agnostic APIs.
