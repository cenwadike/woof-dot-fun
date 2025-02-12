# CW20 Token Factory Smart Contract

## Overview
The Token Factory Smart Contract allows you to create, manage, and query custom tokens on the blockchain. It's designed for ease of use and extensibility.

## Table of Contents
1. [Project Structure](#project-structure)
2. [Setting Up Your Environment](#setting-up-your-environment)
3. [Building the Contract](#building-the-contract)
4. [Running Tests](#running-tests)

## Project Structure

- **src/**: Main contract code.
  - **contract.rs**: Core contract logic.
  - **state.rs**: State definitions.
  - **msg.rs**: Handle messages and queries.
  - **query.rs**: Query implementations.
  - **execute.rs**: Execution logic.
- **Cargo.toml**: Rust project configuration.

## Setting Up Your Environment

1. **Install Rust** from [rust-lang.org](https://www.rust-lang.org/).
2. **Install CosmWasm** following the [CosmWasm installation guide](https://docs.cosmwasm.com/docs/0.16/getting-started/installation).
3. **Clone the Repository**:
    ```sh
        git clone https://github.com/your-repo/token-factory.git
        cd token-factory
    ```

## Building the Contract

Compile the Rust code into WebAssembly (Wasm):

```sh
    cargo wasm
```

## Running Tests

Execute all unit tests:

```sh
    cargo test
```