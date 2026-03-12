# granite

An opinionated library for removing the boilerplate from setting up a window and rendering graphics with `wgpu`.

## Crates

- `granite` (src/granite) holds all the logic to start and run the application.
- `granite-macros` (src/granite-macros) holds procedural macros for simplifying decleration of buffers used for rendering.

## Operations

- To format the code run `cargo fmt --all`
- To check the project for compilation errors, run `cargo check --workspace --all-targets`

## Coding Standards

- Avoid using variable names less than 3 characters and abbreviations in general.  E.g. `position` instead of `pos`, `direction` instead of `dir`.
- Use rust 2024 edition code.
