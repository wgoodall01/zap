# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Structure

This is a monorepo for "zap", a Telegram Mini-App client for OpenShock. The project has three main packages:

- **`pkg/api/`** - Rust backend API using Rocket framework with PostgreSQL/SQLx
- **`pkg/web/`** - React frontend using Vite, TypeScript, and Tanstack Router
- **`pkg/infra/`** - AWS CDK infrastructure code in TypeScript

## Development Commands

### Building and Running

- `make all` - Build all components (api, web, and lambda)
- `make api` - Build the Rust API (`cargo build` in pkg/api)
- `make web` - Build the web frontend (`bun run build` in pkg/web)
- `make api_lambda` - Build API for AWS Lambda deployment
- `make gen` - Generate SQLx offline data and OpenAPI spec

### Frontend (pkg/web)

- `bun run dev` - Start development server on port 3000
- `bun run build` - Build for production (runs `vite build && tsc`)
- `bun run test` - Run tests with Vitest

### Backend (pkg/api)

- `cargo build` - Build the API server
- `cargo sqlx prepare` - Generate SQLx offline query data
- Uses custom build profile `lambda` for AWS deployment optimizations
- When you want to run one-off code---don't run `rustc` or a one-off `.rs` file. Instead, just create a test, and run only that test with `cargo test pkg::path::to::test_name`.

### Infrastructure (pkg/infra)

- `bun run cdk` - Run AWS CDK commands
- `bun run build` - Compile TypeScript
- `bun run test` - Run Jest tests

### Deployment

- `make deploy` - Full deployment (builds lambda + web, deploys migrations, then CDK)

## Key Architecture Notes

- The web frontend generates an OpenAPI client from the API's generated spec at build time
- API uses SQLx with offline query checking for compile-time SQL verification
- Infrastructure is deployed to AWS using CDK with Lambda functions
- Frontend is a Telegram Mini-App using @telegram-apps SDK
- Rust toolchain pinned to version 1.89.0 with rustfmt, clippy, and rust-analyzer

## Dependencies

- **Frontend**: React 19, Vite 6, TypeScript 5.7, Tanstack Router, Radix UI
- **Backend**: Rocket 0.5, SQLx 0.8, AWS Lambda runtime, reqwest
- **Infrastructure**: AWS CDK 2.x, TypeScript

The project uses Bun workspaces and has specific build optimizations for AWS Lambda deployment.
