# MCPify

MCP your OpenAPI files with ease.

## Prerequisites

- Rust and Cargo (install from [rustup.rs](https://rustup.rs/)).

## Usage

```bash
cargo run
cargo build  # debug build.
cargo build --release  # release build.
target/release/mcpify --file openapi.json --output mcp-server
```

## Testing and Linting

```bash
cargo clippy # --fix
cargo fmt
```
