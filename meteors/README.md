# Meteors
Sample game that can be built as a standalone binary package, or packed using [Trunk](https://trunkrs.dev/) to build a 
web page that is ready to be published.

The [main.rs](src/main.rs) file contains the entrypoint for the application, which sets up the engine process and 
configures the environment. The process is then handed off to the [game.rs](src/game.rs) setup function and event
handlers are registered.
