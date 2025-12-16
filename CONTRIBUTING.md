# Contributing to Compute42

Thank you for your interest in contributing to Compute42! This document provides guidelines and instructions for contributing to the project.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/compute42.git
   cd compute42
   ```
3. **Set up the development environment** (see README.md for prerequisites)
4. **Create a branch** for your changes:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

See the [README.md](README.md) for detailed setup instructions. In summary:

1. Install Rust (latest stable)
2. Install Node.js 18+ and npm
3. Run `npm install` in the `app` directory
4. Run `npm run tauri dev` to start development

## Code Style

### Rust

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for common issues
- Write meaningful commit messages

### TypeScript/Vue

- Follow Vue 3 Composition API best practices
- Use TypeScript for type safety
- Run `npm run lint` and `npm run format` before committing
- Use meaningful variable and function names

## Making Changes

1. **Make your changes** in your feature branch
2. **Test your changes** thoroughly
3. **Update documentation** if needed
4. **Commit your changes** with clear, descriptive messages:
   ```bash
   git commit -m "Add feature: description of what you added"
   ```
5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```
6. **Create a Pull Request** on GitHub

## Pull Request Process

1. **Update the README.md** if you're adding new features or changing behavior
2. **Add tests** if applicable
3. **Ensure all tests pass** and the code builds successfully
4. **Request review** from maintainers
5. **Address any feedback** from reviewers

## Commit Message Guidelines

- Use clear, descriptive commit messages
- Start with a verb (Add, Fix, Update, Remove, etc.)
- Reference issue numbers if applicable: `Fix #123: description`
- Keep the first line under 72 characters
- Add more details in the body if needed

Examples:
```
Add: CSV viewer component
Fix: LSP connection timeout issue
Update: Documentation for package installation
```

## Areas for Contribution

We welcome contributions in the following areas:

- **Bug fixes**: Report and fix issues
- **Features**: Add new functionality
- **Documentation**: Improve docs and help files
- **Testing**: Add tests and improve test coverage
- **Performance**: Optimize code and improve performance
- **UI/UX**: Improve user interface and experience
- **Accessibility**: Make the app more accessible

## Reporting Issues

When reporting issues, please include:

- **Description**: Clear description of the issue
- **Steps to reproduce**: Detailed steps to reproduce the issue
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Environment**: OS, Julia version, Compute42 version
- **Logs**: Relevant error messages or logs

## Code of Conduct

- Be respectful and inclusive
- Welcome newcomers and help them learn
- Focus on constructive feedback
- Respect different viewpoints and experiences

## Questions?

If you have questions about contributing, please:

- Check existing issues and pull requests
- Open a new issue with the "question" label
- Reach out to maintainers

Thank you for contributing to Compute42!

