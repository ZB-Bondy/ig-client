<p align="center">
  <img src="doc/images/logo.png" alt="IG Client" width="400" height="200">
</p>
<p style="text-align:center">
  <img src="doc/images/logo.png" alt="IG Client" width="50%" />
</p>

![IG Client](doc/images/logo.png)


[![Dual License](https://img.shields.io/badge/license-MIT%20and%20Apache%202.0-blue)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/ig-client.svg)](https://crates.io/crates/ig-client)
[![Downloads](https://img.shields.io/crates/d/ig-client.svg)](https://crates.io/crates/ig-client)
[![Stars](https://img.shields.io/github/stars/joaquinbejar/ig-client.svg)](https://github.com/joaquinbejar/ig-client/stargazers)

[![Build Status](https://img.shields.io/github/workflow/status/joaquinbejar/ig-client/CI)](https://github.com/joaquinbejar/ig-client/actions)
[![Coverage](https://img.shields.io/codecov/c/github/joaquinbejar/ig-client)](https://codecov.io/gh/joaquinbejar/ig-client)
[![Dependencies](https://img.shields.io/librariesio/github/joaquinbejar/ig-client)](https://libraries.io/github/joaquinbejar/ig-client)

# IG Client v0.1.0: Rust Framework for IG Broker Operations

## Table of Contents
1. [Introduction](#introduction)
2. [Features](#features)
3. [Project Structure](#project-structure)
4. [Setup Instructions](#setup-instructions)
5. [Library Usage](#library-usage)
6. [Usage Examples](#usage-examples)
7. [Testing](#testing)
8. [Contribution and Contact](#contribution-and-contact)

## Introduction

IG Client is a comprehensive Rust framework for interacting with the IG broker API. This versatile toolkit enables traders, developers, and financial professionals to:

## Features

1. **Order Execution**: Execute orders on the IG platform programmatically.
2. **Order Management**: Manage existing orders, including modifications and cancellations.
3. **Market Data**: Retrieve real-time and historical market data for various instruments.
4. **Account Information**: Access account details, balances, and trading history.
5. **Asynchronous Operations**: Utilize async/await for efficient API interactions.
6. **Error Handling**: Robust error handling with anyhow for clear and informative error messages.
7. **Logging and Tracing**: Comprehensive logging and tracing with tracing and tracing-subscriber.
8. **Unit Testing**: Extensive unit tests with pretty_assertions and assert-json-diff for clear test outputs.

## Project Structure

The project is structured as follows:

1. **Configuration** (`src/config.rs`): Configuration settings for the IG Client.

2. **Application Layer** (`src/application/`):
    - **API** (`src/application/api/`):
        - **Client** (`src/application/api/client.rs`): Main client interface for interacting with the IG API.
        - **Endpoints** (`src/application/api/endpoints.rs`): Definitions of API endpoints.
    - **Models** (`src/application/models/`):
        - **Account** (`src/application/models/account.rs`): Account-related data structures.
        - **Market** (`src/application/models/market.rs`): Market-related data structures.
        - **Order** (`src/application/models/order.rs`): Order-related data structures.
    - **Services** (`src/application/services/`):
        - **Market Data** (`src/application/services/market_data.rs`): Functions for retrieving and processing market data.
        - **Order Execution** (`src/application/services/order_execution.rs`): Functions for executing orders.
        - **Order Management** (`src/application/services/order_management.rs`): Functions for managing existing orders.

3. **Presentation Layer** (`src/presentation/`):
    - **Encryption** (`src/presentation/encryption.rs`): Data encryption utilities.
    - **Serialization** (`src/presentation/serialization.rs`): Data serialization and deserialization utilities.

4. **Session Management** (`src/session/`):
    - **Authentication** (`src/session/auth.rs`): Authentication and session management.

5. **Transport Layer** (`src/transport/`):
    - **HTTP Client** (`src/transport/http_client.rs`): Core implementation of the HTTP client for interacting with the IG API.

6. **Utilities** (`src/utils/`):
    - **Error Handling** (`src/utils/error.rs`): Custom error types and error handling utilities.

7. **Tests** (`tests/`): Directory containing all unit tests.

8. **Benchmarks** (`benches/`): Directory containing benchmark tests.

9. **Examples** (`examples/`): Directory containing usage examples.

## Setup Instructions

1. Clone the repository:
```shell
git clone https://github.com/joaquinbejar/ig-client.git
cd ig-client
```

2. Build the project:
```shell
cargo build
```

3. Run tests:
```shell
cargo test
```

4. Format the code:
```shell
cargo fmt
```

5. Run linting:
```shell
cargo clippy
```

## Library Usage

To use the library in your project, add the following to your `Cargo.toml`:

```toml
[dependencies]
ig-client = { git = "https://github.com/joaquinbejar/ig-client.git" }
```

## Usage Examples

Here are some examples of how to use the library for interacting with the IG broker:

```rust
use ig_client::IGHttpClient;
use ig_client::order::Order;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = IGHttpClient::new("YOUR_API_KEY", "YOUR_ACCOUNT_ID");
    
    // Place a new order
    let order = Order::new("EURUSD", "BUY", 1000);
    let result = client.place_order(order).await?;
    println!("Order placed: {:?}", result);
    
    // Get account information
    let account_info = client.get_account_info().await?;
    println!("Account balance: {}", account_info.balance);

    Ok(())
}
```

## Testing

To run unit tests:
```shell
cargo test
```

To run tests with coverage:
```shell
cargo tarpaulin
```

## Contribution and Contact

We welcome contributions to this project! If you would like to contribute, please follow these steps:

1. Fork the repository.
2. Create a new branch for your feature or bug fix.
3. Make your changes and ensure that the project still builds and all tests pass.
4. Commit your changes and push your branch to your forked repository.
5. Submit a pull request to the main repository.

If you have any questions, issues, or would like to provide feedback, please feel free to contact the project maintainer:

**Joaquín Béjar García**
- Email: jb@taunais.com
- GitHub: [joaquinbejar](https://github.com/joaquinbejar)

We appreciate your interest and look forward to your contributions!