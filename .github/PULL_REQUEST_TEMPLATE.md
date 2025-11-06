# Pull Request

## Description
<!-- Brief description of changes -->

## Type of Change
<!-- Mark relevant options with [x] -->
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to change)
- [ ] Documentation update
- [ ] Refactoring (no functional changes)
- [ ] Performance improvement

## Related Issues
<!-- Link to related issues: Closes #123, Relates to #456 -->

## Testing
<!-- Describe testing performed -->
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] E2E tests added/updated (for frontend changes)
- [ ] Manual testing performed

## Documentation Checklist
<!-- IMPORTANT: Documentation must be updated for API changes -->
- [ ] **Public API Documentation**: All new/modified public APIs have `///` doc comments
- [ ] **Module Documentation**: New modules have `//!` module-level documentation
- [ ] **Examples**: Complex APIs include code examples (doctests)
- [ ] **Cross-references**: Related types/functions are linked using `[Type]` or `[crate::module::Type]`
- [ ] **Breaking Changes**: If breaking changes exist, affected documentation is updated
- [ ] **RUNBOOK.md**: Updated if new commands/procedures are added
- [ ] **Doctest Validation**: All code examples compile and run (`cargo test --doc`)
- [ ] **Link Validation**: No broken documentation links (`cargo rustdoc -- -D warnings`)

### Documentation Quality Guidelines
- Use clear, concise descriptions (1-2 sentences minimum)
- Include `# Arguments`, `# Returns`, `# Errors`, `# Panics`, `# Examples` sections where applicable
- Use markdown formatting: headings, lists, code blocks
- Ensure all examples are self-contained and executable
- For non-runnable examples, use appropriate attributes: `no_run`, `ignore`, `compile_fail`

### Breaking Change Impact
If this PR includes breaking changes, ensure:
- [ ] All affected API documentation is updated to reflect new behavior
- [ ] Migration guide or upgrade notes are added (if significant)
- [ ] Deprecated APIs are marked with `#[deprecated]` attribute
- [ ] Version compatibility notes are added to module documentation

## CI/CD Checklist
<!-- All checks must pass before merge -->
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `npm run lint` passes (for JavaScript changes)
- [ ] `npm run test:e2e` passes (for web UI changes)
- [ ] `cargo doc --workspace --no-deps` builds successfully
- [ ] `cargo test --workspace --doc` passes (doctest validation)

## Reviewer Notes
<!-- Additional context for reviewers -->
