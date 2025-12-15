## CRITICAL: Test Integrity Rules

**NEVER MODIFY TESTS TO MAKE THEM PASS - unless tests are provably broken.**

### Ironclad Test Integrity Rules:

1. **Tests are Sacred**: Tests define the expected behavior. If a test fails, assume that the code is wrong, not the test.

2. **Only Three Valid Reasons to Change a Test**:

   - **Provably incorrect logic**: The test itself contains a mathematical or logical error
   - **Invalid assumptions**: The test assumes behavior that contradicts the specification
   - **Typos/syntax errors**: Clear mistakes in test code (like wrong variable names)

3. **Required Evidence for Test Changes**:

   - **Written justification**: Must document exactly what was wrong with the original test
   - **Reference to specification**: Show how the test contradicts documented behavior
   - **Alternative verification**: Demonstrate the correct behavior through independent means

4. **Forbidden "Fixes"**:

   - Changing expected values because they don't match actual output
   - Reducing test scope because full test fails
   - Simplifying test cases because they're "too complex"
   - Removing assertions because they fail
   - Making tests "more realistic" when they expose bugs

5. **Mandatory Process When Tests Fail**:
   - **Step 1**: Assume the test is correct and the implementation is wrong
   - **Step 2**: Investigate why the implementation doesn't meet the test's expectations
   - **Step 3**: Fix the implementation to satisfy the test
   - **Step 4**: Only if Step 3 is impossible, then question if the test is wrong
   - **Step 5**: If changing a test, require explicit approval with written justification

### Red Flag Phrases That Should Trigger Immediate Stop:

- "Let me simplify this test..."
- "This test is too complex, let me make it more realistic..."
- "The test expects X but that's not how it actually works..."
- "Let me adjust the expected values..."
- "This test is causing issues, let me fix it..."

### Correct Mindset:

- **Tests are the specification in executable form**
- **Failing tests reveal implementation gaps, not test problems**
- **Complex tests often catch the most important bugs**
- **Test failures are valuable information about what needs to be built**

## Development Commands

### Tools

- **Build system**: `cargo` (Rust's package manager and build tool)
- **Code formatting**: `cargo fmt` (run before committing)
- **Linting**: `cargo clippy` (run before committing; fix all warnings)

### Testing

- Unit tests go in the same file as the code, in a `#[cfg(test)]` module
- Integration tests go in: `tests/`

For ad-hoc testing, don't write to temporary locations, like `/tmp`, but instead write your scripts to `tmp/` in the project directory. These should never be committed.

### Examples: Running Tests

```bash
# Run all tests
cargo test

# Run specific test file (integration test)
cargo test --test test_name

# Run with verbose output
cargo test -- --nocapture

# Run specific test function
cargo test test_function_name

# Run tests in a specific module
cargo test module_name::
```

### Code Formatting

```bash
# Format all Rust files
cargo fmt

# Check formatting without changing files
cargo fmt --check
```

### Linting

```bash
# Run clippy with warnings as errors
cargo clippy -- -D warnings

# Run clippy and apply automatic fixes
cargo clippy --fix
```

### Development Setup

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Add a new dependency
cargo add <crate-name>

# Add a development dependency
cargo add --dev <crate-name>
```

## Implementation Guidelines

### Project Rules

1. **No workarounds** - Fix root causes, not symptoms
2. **Proper error handling** - Use `Result<T, E>` for recoverable errors; avoid `.unwrap()` in library code
3. **No backwards compatibility** - Focus on clean design for new features
4. **No backup files** - Git handles versioning, no suffixes or backup copies
5. **Direct communication** - No unnecessary affirmations or compliments
6. **Frequent commits** - Commit working code frequently, small logical changes
7. **No regressions** - all existing tests must pass
8. **No unsafe without justification** - Document why `unsafe` is necessary if used

### When Adding New Features

1. Keep modules small and single-purpose
2. Expose stable public APIs; keep internals private
3. Always include unit tests alongside code changes
4. Use `#[must_use]` for functions where ignoring the return value is likely a bug

### Process

- Don't back files up by copying! We use git for versioning.
- For each new development stage, create a new git branch first.
- We practice TDD:
  - write tests first that demonstrate the desired behaviour
  - **pause for human review of the tests**
  - progress the implementation until the tests succeed.
  - NEVER tweak a test to "fit" the behaviour, unless the test is demonstrably broken.
- Maintain progress in docs/TODO.md
- NEVER EVER CHANGE THE DEFAULT BRANCH ON GIT OR GITHUB!
- When creating PRs or commits, DO NOT mention Claude, Anthropic, or AI assistance in the message
- NEVER use `--no-verify` when committing! Always let pre-commit hooks run and fix any issues they find

### GitHub Issue Workflow

In any git and GitHub messaging (commit messages, PR messages, issues, comments etc), we maintain a terse, professional tone:

1. **Never make unproven claims**: don't make claims about the validity, effectiveness or awesomeness of your changes in a commit or message. By definition, that is determined by the CI results, which you can't see yet. Explain what was done, and why.
2. **Never use emoji symbols**: we're not 14-year-olds on Instagram here. No green ticks, no red crosses, no smileys, no symbols.
3. **Brevity**: issues and commit messages are written for co-workers. Respect their time. Obviously, be complete, but express yourself in a professional, concise tone.

Follow this process for each GitHub issue:

1. **Pick an issue** - Note its ID number
2. **Create branch** - Name format: `{ID}-{slug-derived-from-issue-title}`
   - Example: `9-parser-grammar-basic-terms`
3. **Write tests FIRST** - STOP after writing tests for human review
4. **Commit approved tests** - Only after review approval
5. **Implement until tests pass** - Make the tests green
6. **Run complete test suite** - No regressions tolerated!
7. **Create PR** - Make an orderly PR, squashing commits if necessary. DON'T mention Claude or AI in the PR message
8. **Verify CI** - Ensure all CI tests pass fully
9. **Await PR review** - Wait for human review
10. **Merge and update** - After approval, merge PR and update the epic

- Don't use /tmp and other locations outside the current repository
- You MUST stop for reviews before ANY implementation
