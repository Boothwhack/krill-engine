# Krill game engine
This project is built with technologies that support compiling to a multitude of platforms. The Rust compiler has built-in support for compiling to WebAssembly, which allows for near-native performance on the web.

## Demo
A demo is available [here](https://boothwhack.github.io/krill-engine/) in the form of a recreation of a classic arcade game.

The code for this demo is split into three components. There's the core game application code which is written using platform-agnostic APIs. This core gets invoked by two separate platform-specific executable targets; one for web browsers and one for desktop computers.

These components are located in the [game](game), [game-web](game-web) and [game-desktop](game-desktop) directories respectively.