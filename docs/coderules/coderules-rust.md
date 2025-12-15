# Code Rules and Processes for this project

**KEY RULES**:

- NEVER CLAIM THAT SOMETHING IS COMPLETE IF THERE ARE REGRESSIONS. RUN THE FULL TEST SUITE BEFORE AND AFTER EACH WORK UNIT.
- NEVER USE `--no-verify` WITH GIT!
- **ALL CODE MUST PASS `cargo clippy` AND `cargo fmt --check`** before committing

## Test Locations

- **Unit tests**: Place in the same file as the code, in a `#[cfg(test)]` module at the bottom
- **Integration tests**: Place in `tests/` directory
- Test functions should be named descriptively: `test_*` or use `#[test]` attribute

## Developing in Rust

- Never do "fallback" programming in terms of requirements: if you expect a dependency, fail immediately if it's not present
- Use `thiserror` for library errors, `anyhow` for application errors where appropriate
- **Error handling**:
  - Use `Result<T, E>` for recoverable errors, never panic for expected failure cases
  - Avoid `.unwrap()` and `.expect()` in library code; acceptable in tests and when truly impossible to fail
  - Propagate errors with `?` operator
- Use the latest stable Rust edition (2021)
- Backwards compatibility is NOT a goal, neither in terms of Rust, nor in terms of this project's code itself
- Prefer explicit types over excessive type inference where it aids readability
- Use `#[must_use]` for functions where ignoring the return value is likely a bug

## Code Quality

- **CRITICAL**: Run `cargo clippy -- -D warnings` before committing; treat all warnings as errors
- Run `cargo fmt` before committing to ensure consistent formatting
- Prefer iterator methods (`.map()`, `.filter()`, etc.) over manual loops where appropriate
- Use `const` for compile-time constants, not `static` unless interior mutability is needed
- Avoid `unsafe` unless absolutely necessary; document why it's safe if used

## Debugging

- **CRITICAL**: Always identify root causes of failures. Do NOT treat the symptoms of failures.
- Use `dbg!()` macro for quick debugging, but remove before committing
- Use `tracing` or `log` crate for permanent logging needs

## Process

- Don't back files up by copying! We use git for versioning.
- For each new development stage, create a new git branch first.
- We practice TDD:
  - Write tests first that demonstrate the desired behaviour
  - Pause for human review of the tests
  - Progress the implementation until the tests succeed.
  - NEVER tweak a test to "fit" the behaviour, unless the test is demonstrably broken.
  - Once a test set has been reviewed and approved, that's a contract: do NOT skip or change without re-approval. All approved tests MUST pass before PR.
  - Before opening a PR, you MUST ensure that the full test suite is green.
  - Review any `#[ignore]` tests and ensure they are documented.
  - Fix any compiler warnings.
- Maintain progress in docs/TODO-X.md files
- Don't use /tmp and other locations outside the current repository: use the tmp/ directory in the repository dir instead, provided for this purpose
- If you create temporary scripts for debugging, remove them after use, and ensure not committed to git

## GitHub Workflow

- **NEVER EVER CHANGE THE DEFAULT BRANCH ON GIT OR GITHUB!**
- When creating PRs or commits, **DO NOT** mention Claude, Anthropic, or AI assistance in the message

**Note**: Don't bulk-add changes to git! Add modifications, additions and deletions individually, based on the knowledge of what you have actually done. That makes it easier for the human to follow, too. Doing it this way reduces the chance that unintended changes makes it into git.

Follow this process for each GitHub issue:

1. **Pick an issue** - Note its ID number
2. **Create branch** - Name format: `{ID}-{slug-derived-from-issue-title}`
   - Example: `9-parser-grammar-basic-terms`
3. **Write tests FIRST** - STOP after writing tests for human review
4. **Commit approved tests** - Only after review approval
5. **Implement until tests pass** - Make the tests green
6. **Run complete test suite** - No regressions tolerated!
7. **Create PR** - Make an orderly PR, squashing commits if necessary.
8. **Verify CI** - Ensure all CI tests pass fully
9. **Await PR review** - Wait for human review
10. **Merge** - After approval, merge PR and verify that tests complete in CI
11. **Maintain issues** Maintain issues by checking boxes where relevant after every commit. If all boxes are ticked, close the issues.
12. **Maintain epics** Update the Epic issue where relevant by ticking any boxes as issues are closed. If all sub-issues are closed, also close the epic.

In any git and GitHub messaging (commit messages, PR messages, issues, comments etc), we maintain a terse, professional tone:

1. **Never make unproven claims**: don't make claims about the validity, effectiveness or awesomeness of your changes in a commit or other message. By definition, that is determined by the CI results, which you can't see yet. Explain what was done, and why. Be modest and factual.
2. **Never use emoji symbols**: we're not 14-year-olds on Instagram here. No green ticks, no red crosses, no smileys, no symbols.
3. **Don't use bold text**: don't embellish or add emphasis with bold or italic text.
4. **Brevity**: issues and commit messages are written for co-workers. Respect their time. Obviously, be complete, but express yourself in a professional, concise tone.
5. **UK English**: we use UK English spelling throughout.
