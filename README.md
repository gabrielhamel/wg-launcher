# wg-launcher-cli

A TypeScript CLI tool for interacting with the Wargaming Update Service (WGUS) API. This tool fetches game update and patch information for World of Tanks EU production environment.

## Features

- 🔧 **Type-safe XML API client** for Wargaming Update Service
- ⚡ **Effect-TS based architecture** with functional error handling
- 📋 **Runtime schema validation** for API responses
- 🎯 **CLI interface** with subcommands using @effect/cli
- 🛡️ **Robust error handling** with tagged errors and proper propagation

## Prerequisites

- Node.js 18 or higher

## Installation

```bash
npm install
```

## Usage

### Development Mode

Run the CLI with auto-reload during development:

```bash
npm run dev
```

### Observability with OpenTelemetry

The CLI includes built-in OpenTelemetry instrumentation for tracing and observability.

#### Starting the Telemetry Collector

Launch the OTEL TUI collector to visualize traces:

```bash
npm run telemetry-collector
```

This starts a Docker container running [otel-tui](https://github.com/ymtdzzz/otel-tui), which provides a terminal UI for viewing OpenTelemetry traces in real-time.

#### Configuration

Set the `OPEN_TELEMETRY_SPAN_EXPORT_URL` environment variable to enable trace export:

```bash
export OPEN_TELEMETRY_SPAN_EXPORT_URL=http://localhost:4318
npm run dev
```

When configured, the CLI will automatically export trace spans to the specified endpoint. If the variable is not set, telemetry remains disabled.

## Development

### Running Tests

```bash
# Run once
npm test

# Watch mode
npm run test:watch
```

### Code Quality

```bash
# Check everything
npm run ci

# Individual checks
npm run format:check
npm run lint:check
npm run type:check
```

## Technology Stack

- **[Effect-TS](https://effect.website/)** - Functional effect system for TypeScript
- **[@effect/cli](https://effect.website/docs/guides/cli-applications)** - CLI framework built on Effect
- **[@effect/platform](https://effect.website/docs/guides/platform)** - Cross-platform abstractions
- **[@effect/opentelemetry](https://effect.website/docs/guides/observability/telemetry)** - OpenTelemetry integration for tracing
- **[Vitest](https://vitest.dev/)** - Testing framework
- **[ESLint](https://eslint.org/)** - Linting
- **[dprint](https://dprint.dev/)** - Code formatting

### Key Design Patterns

- **Effect Services**: Dependency injection using `Effect.Service`
- **Tagged Errors**: Type-safe error handling with `Effect.catchTags`
- **Runtime Validation**: Schema validation with `Schema.decodeUnknown`
- **Functional Composition**: Effect generators for readable async code
