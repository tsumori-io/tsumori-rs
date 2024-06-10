# Tsumori

A monolithic repository for Tsumori.

The repo contains a CLI to run the Tsumori service; additionally contains crates for relvant tsumori operations.

## Quickstart

### Prerequisites

- Install Rust 1.76.0 (nightly)
  - `rustup override set 1.76.0`

### Run

```sh
cargo run -- server
```

## Docker setup

### Build

```sh
docker build -t tsumori-io:tsumori -f Dockerfile .
```

### Run

```sh
docker compose up
```

## CLI usage

```sh
cargo run -- --help
```

```
tsumori-rs

Usage: tsumori <COMMAND>

Commands:
  server  Run http server
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Server

```sh
cargo run -- server --help
```

```
Run http server

Usage: tsumori server [OPTIONS]

Options:
  -p, --port <PORT>                  The port to listen on [default: 8080]
  -r, --req-timeout <TIMEOUT>        The request timeout in seconds [default: 10]
  -m, --metrics-port <METRICS_PORT>  The port to listen on for metrics [default: 9090]
  -l, --log-level <LOG_LEVEL>        Log level [default: info] [possible values: trace, debug, info, warn, error]
  -h, --help                         Print help
```

## Bridging

### Supported adapters

- Across
- DeBridge (DLN)
