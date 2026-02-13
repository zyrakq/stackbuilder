## Brief overview

Strict Rust development workflow with zero tolerance for compiler and linter warnings. Code must pass all checks without any warnings.

## Communication style

- Respond concisely and directly, avoid unnecessary words
- Analyze first, then act when executing tasks
- When status is requested, provide specific progress information

## Development workflow

- Always run `cargo check` before each commit
- Check `cargo clippy` before task completion and fix all warnings
- Use `cargo test` for testing changes
- Use `cargo build` or `cargo run --bin stackbuilder build` for building
- Test changes using examples from `examples/` directory

## Code quality requirements

- Code must compile without `cargo check` warnings
- Code must pass `cargo clippy` without issues
- Use `cargo clippy --fix` for automatic fixing of simple problems
- Follow Clippy recommendations for code improvements

## Git workflow

- Commit format: verb in infinitive form (Fix, Update, Refactor, etc.)
- Commits must be atomic and logically complete
- Move to next task immediately after commit
