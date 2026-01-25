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
- npm or compatible package manager

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
- **[fast-xml-parser](https://github.com/NaturalIntelligence/fast-xml-parser)** - XML parsing library
- **TypeScript** - Type-safe JavaScript
- **[Vitest](https://vitest.dev/)** - Testing framework
- **[ESLint](https://eslint.org/)** - Linting
- **[dprint](https://dprint.dev/)** - Code formatting

## Architecture

The project follows a **clean architecture** pattern with clear separation of concerns:

- **`core/`** - Domain logic
- **`infra/`** - Infrastructure layer

### Key Design Patterns

- **Effect Services**: Dependency injection using `Effect.Service`
- **Tagged Errors**: Type-safe error handling with `Effect.catchTags`
- **Runtime Validation**: Schema validation with `Schema.decodeUnknown`
- **Functional Composition**: Effect generators for readable async code

## License

MIT

## Author

Gabriel Hamel <gabriel.hamel.pro@gmail.com>

## Contributing

This is a personal project. For issues or suggestions, please contact the author.
