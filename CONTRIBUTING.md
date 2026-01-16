# Contributing to SSO-1

Thank you for your interest in contributing to SSO-1!

## Code of Conduct

This project adheres to a code of conduct. By participating, you are expected to uphold this code.

## How to Contribute

### Reporting Issues

- Check existing issues before creating a new one
- Use issue templates when available
- Include relevant details: version, environment, steps to reproduce

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Run tests (`make test`)
5. Run linters (`make lint`)
6. Commit with clear messages
7. Push and create a Pull Request

### Development Setup

```bash
# Clone your fork
git clone https://github.com/your-username/sso-1.git
cd sso-1

# Set up development environment
make setup-dev

# Build all components
make build

# Run tests
make test
```

### Code Style

**Rust (On-Chain)**:
- Follow Rust standard formatting (`cargo fmt`)
- Use Clippy for linting (`cargo clippy`)
- Document public APIs

**Python (Off-Chain)**:
- Use Ruff for formatting and linting
- Type hints required for public functions
- Docstrings for modules, classes, and functions

### Commit Messages

Use conventional commits:
- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation
- `test:` Tests
- `refactor:` Code refactoring
- `chore:` Maintenance

### Testing

- All new features must include tests
- Integration tests for cross-component changes
- Maintain or improve code coverage

## Security

For security issues, please see [SECURITY.md](./SECURITY.md) or email security@your-org.com.

## License

By contributing, you agree that your contributions will be licensed under Apache 2.0.
