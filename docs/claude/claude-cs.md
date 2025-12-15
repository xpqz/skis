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

- **Build system**: `dotnet` CLI
- **Code formatting**: `dotnet format` (run before committing)
- **Linting**: Roslyn analysers (enable in project; treat warnings as errors)

### Testing

- Test projects should be named `ProjectName.Tests`
- Use xUnit, NUnit, or MSTest consistently
- Integration tests go in: `ProjectName.IntegrationTests`

For ad-hoc testing, don't write to temporary locations, like `/tmp`, but instead write your scripts to `tmp/` in the project directory. These should never be committed.

### Examples: Running Tests

```bash
# Run all tests
dotnet test

# Run specific test project
dotnet test tests/ProjectName.Tests

# Run with verbose output
dotnet test --verbosity normal

# Run specific test by name
dotnet test --filter "FullyQualifiedName~TestMethodName"

# Run tests with coverage
dotnet test --collect:"XPlat Code Coverage"
```

### Code Formatting

```bash
# Format all files
dotnet format

# Check formatting without changing files
dotnet format --verify-no-changes
```

### Development Setup

```bash
# Restore dependencies
dotnet restore

# Build the solution
dotnet build

# Build in release mode
dotnet build --configuration Release

# Add a new dependency
dotnet add package <package-name>
```

## Implementation Guidelines

### Project Rules

1. **No workarounds** - Fix root causes, not symptoms
2. **Enable nullable reference types** - Use `<Nullable>enable</Nullable>` in all projects
3. **No backwards compatibility** - Focus on clean design for new features
4. **No backup files** - Git handles versioning, no suffixes or backup copies
5. **Direct communication** - No unnecessary affirmations or compliments
6. **Frequent commits** - Commit working code frequently, small logical changes
7. **No regressions** - all existing tests must pass
8. **Use modern C# features** - File-scoped namespaces, primary constructors, records where appropriate

### When Adding New Features

1. Keep classes small and single-purpose
2. Expose stable public APIs; use `internal` for implementation details
3. Always include unit tests alongside code changes
4. Use `sealed` for classes not designed for inheritance

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
