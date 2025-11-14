# AI-DLC and Spec-Driven Development

Kiro-style Spec Driven Development implementation on AI-DLC (AI Development Life Cycle)

## Project Context

### Paths
- Steering: `.kiro/steering/`
- Specs: `.kiro/specs/`

### Steering vs Specification

**Steering** (`.kiro/steering/`) - Guide AI with project-wide rules and context
**Specs** (`.kiro/specs/`) - Formalize development process for individual features

### Active Specifications
- Check `.kiro/specs/` for active specifications
- Use `/kiro:spec-status [feature-name]` to check progress

## Development Guidelines
- Think in English, generate responses in English

## Minimal Workflow
- Phase 0 (optional): `/kiro:steering`, `/kiro:steering-custom`
- Phase 1 (Specification):
  - `/kiro:spec-init "description"`
  - `/kiro:spec-requirements {feature}`
  - `/kiro:validate-gap {feature}` (optional: for existing codebase)
  - `/kiro:spec-design {feature} [-y]`
  - `/kiro:validate-design {feature}` (optional: design review)
  - `/kiro:spec-tasks {feature} [-y]`
- Phase 2 (Implementation): `/kiro:spec-impl {feature} [tasks]`
  - `/kiro:validate-impl {feature}` (optional: after implementation)
- Progress check: `/kiro:spec-status {feature}` (use anytime)

## Development Rules
- 3-phase approval workflow: Requirements → Design → Tasks → Implementation
- Human review required each phase; use `-y` only for intentional fast-track
- Keep steering current and verify alignment with `/kiro:spec-status`

## Steering Configuration
- Load entire `.kiro/steering/` as project memory
- Default files: `product.md`, `tech.md`, `structure.md`
- Custom files are supported (managed via `/kiro:steering-custom`)

## Issue Fix Workflow

For direct bug fixes and smaller improvements (not requiring full spec-driven development):

### Phase 1: Investigation & Planning
1. **Issue Analysis**: Read GitHub issue details
   ```bash
   gh issue view <issue-number>
   ```
2. **Codebase Investigation**: Use Plan subagent for thorough analysis
   - Identify all affected files and functions
   - Find all panic!/unwrap()/expect() locations
   - Analyze impact on callers and dependencies
   - Review existing error handling patterns
3. **Present Plan**: Use ExitPlanMode to present comprehensive plan to user
   - Include all changes needed
   - Assess risks and breaking changes
   - Get user approval before proceeding

### Phase 2: Implementation
1. **Branch Creation**: Create feature branch from main
   ```bash
   git checkout -b issue-<number>-<description>
   ```
2. **Code Changes**: Implement fixes following the plan
   - Update error types if needed
   - Change function signatures (Result types)
   - Update all callers
   - Fix related issues (logger, CLI, etc.)
3. **Quality Checks**: Run all validation steps
   ```bash
   cargo build --release
   cargo test
   cargo clippy --all-targets
   ```

### Phase 3: Commit & PR
1. **Stage Changes**: Add modified files
   ```bash
   git add <files>
   ```
2. **Commit**: Use conventional commit format
   ```bash
   git commit -m "fix(module): description (#issue-number)"
   ```
   - Include detailed change summary
   - Reference issue with "Fixes #<number>"
   - Add Claude Code attribution
3. **Push & PR**: Create pull request
   ```bash
   git push -u origin <branch-name>
   gh pr create --title "..." --body "..." --base main
   ```

### Best Practices
- Always use Plan subagent for comprehensive codebase analysis
- Get user approval on plan before implementation
- Run all quality checks (build, test, clippy) before committing
- Follow pre-commit hook requirements (rustfmt)
- Include detailed PR descriptions with impact assessment

