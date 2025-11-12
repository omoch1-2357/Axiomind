# Implementation Plan

## Task Format Template

Use whichever pattern fits the work breakdown:

### Major task only
- [ ] {{NUMBER}}. {{TASK_DESCRIPTION}}{{PARALLEL_MARK}}
  - {{DETAIL_ITEM_1}} *(Include details only when needed. If the task stands alone, omit bullet items.)*
  - _Requirements: {{REQUIREMENT_IDS}}_

### Major + Sub-task structure
- [ ] {{MAJOR_NUMBER}}. {{MAJOR_TASK_SUMMARY}}
- [ ] {{MAJOR_NUMBER}}.{{SUB_NUMBER}} {{SUB_TASK_DESCRIPTION}}{{SUB_PARALLEL_MARK}}
  - {{DETAIL_ITEM_1}}
  - {{DETAIL_ITEM_2}}
  - _Requirements: {{REQUIREMENT_IDS}}_ *(IDs only; do not add descriptions or parentheses.)*

> **Parallel marker**: Append ` (P)` only to tasks that can be executed in parallel. Omit the marker when running in `--sequential` mode.
>
> **Optional test coverage**: When a sub-task is deferrable test work tied to acceptance criteria, mark the checkbox as `- [ ]*` and explain the referenced requirements in the detail bullets.

---

## Implementation Tasks

### Phase 1: Quick Wins (Documentation and Warning System)

- [ ] 1. Command registry cleanup
- [ ] 1.1 (P) Remove non-existent commands from COMMANDS array
  - Remove "serve" from COMMANDS constant in `rust/cli/src/lib.rs`
  - Remove "train" from COMMANDS constant
  - Verify COMMANDS array only includes commands present in Commands enum
  - _Requirements: 5, 6_

- [ ] 1.2 (P) Implement consistent warning display system
  - Create warning formatter that outputs to stderr with "WARNING:" prefix
  - Create parameter unused warning formatter accepting parameter name
  - Create demo mode output tagging function that appends status indicators
  - Ensure all warning functions accept generic Write trait for testability
  - _Requirements: 2, 3, 4, 10_

- [ ] 1.3 (P) Update documentation with implementation status
  - Add "Implementation Status" column to `docs/CLI.md` command table
  - Mark "play --vs ai" as PARTIAL with limitation description
  - Mark "replay" as PARTIAL with "count only" description
  - Mark "eval" as PARTIAL with "random placeholder" description
  - Mark "serve" as PLANNED with "not available" status
  - Mark "train" as PLANNED with "not available" status
  - Include manual workaround for running web server
  - _Requirements: 7_

- [ ] 2. Add placeholder warnings to existing commands
- [ ] 2.1 (P) Add AI play mode placeholder warning
  - Display warning at command start: "WARNING: AI opponent is a placeholder that always checks. Use for demo purposes only."
  - Append "[DEMO MODE]" to AI action output messages
  - Ensure warning appears before any game output
  - _Requirements: 2, 10_

- [ ] 2.2 (P) Add replay functionality warning
  - Display warning: "Note: Full visual replay not yet implemented. This command only counts hands in the file."
  - Change output message from "Replayed: N hands" to "Counted: N hands in file"
  - Remove --speed parameter from command signature
  - _Requirements: 3, 10_

- [ ] 2.3 (P) Add eval placeholder warning
  - Display warning at command start: "WARNING: This is a placeholder returning random results. AI parameters are not used. For real simulations, use 'axm sim' command."
  - Warn that --ai-a and --ai-b parameters are currently unused
  - Append "[RANDOM RESULTS - NOT REAL AI COMPARISON]" to summary output
  - _Requirements: 4, 10_

### Phase 2: Fix Human Play Mode

- [ ] 3. Implement stdin reading and input parsing
- [ ] 3.1 Build stdin reader function
  - Create function accepting generic BufRead trait for stdin reading
  - Implement blocking line-buffered input reading
  - Handle EOF gracefully by returning None
  - Trim whitespace from input lines
  - Return None for empty strings after trimming
  - _Requirements: 1_

- [ ] 3.2 Build input parser for player actions
  - Create ParseResult enum with Action, Quit, and Invalid variants
  - Implement case-insensitive parsing for fold, check, call commands
  - Parse "bet <amount>" and "raise <amount>" patterns with positive integer validation
  - Recognize quit commands: "q", "quit" (case-insensitive)
  - Return descriptive error messages for unrecognized input
  - _Requirements: 1_

- [ ] 3.3 Integrate stdin reading into play command handler
  - Refactor play command handler to accept BufRead and Write traits
  - Create main game loop that displays game state before each action
  - Call stdin reader for user input in human mode
  - Call input parser to convert text to PlayerAction
  - Handle quit commands by displaying session summary and exiting gracefully
  - Handle invalid input by showing error message and re-prompting without terminating
  - Display current cards, pot, stack sizes, and board state before each prompt
  - Progress through complete hand including showdown with winner display
  - _Requirements: 1_

### Phase 3: Testing Infrastructure

- [ ] 4. Add behavioral tests for interactive commands
- [ ] 4.1 Test human play stdin blocking behavior
  - Spawn play command with piped stdin using std::process::Command
  - Verify process remains running after short delay (confirms blocking)
  - Send valid input through pipe and verify action processed
  - Verify game state progresses after input
  - _Requirements: 8_

- [ ] 4.2 Test input parsing and error handling
  - Test valid actions (fold, check, call, bet 100, raise 50) produce correct PlayerAction
  - Test quit commands (q, quit) result in graceful exit
  - Test invalid input triggers error message and re-prompt
  - Test EOF handling causes graceful termination
  - Test empty input is handled without crashing
  - _Requirements: 8_

- [ ] 4.3 Test placeholder warning display
  - Verify AI play mode displays placeholder warning to stderr
  - Verify eval command displays placeholder warning prominently
  - Verify replay command displays "Note:" warning about missing functionality
  - Verify all warnings use consistent "WARNING:" prefix format
  - _Requirements: 8, 10_

- [ ] 4.4 Test command output accuracy
  - Verify replay outputs "Counted: N hands in file" not "Replayed"
  - Verify AI play mode includes "[DEMO MODE]" in action messages
  - Verify eval output includes "[RANDOM RESULTS - NOT REAL AI COMPARISON]" tag
  - Verify parameter unused warnings appear when applicable
  - _Requirements: 8_

- [ ]*4.5 Add comprehensive edge case coverage for human play
  - Test negative bet amounts are rejected with clear error
  - Test bet amounts exceeding stack size are handled
  - Test multiple consecutive invalid inputs don't crash
  - Test mid-hand quit displays partial session statistics
  - Test complete hand progression through all betting rounds
  - Verify showdown displays final hands and correct winner
  - _Requirements: 1_

### Phase 4: Process and Quality Improvements

- [ ] 5. Establish command implementation standards
- [ ] 5.1 (P) Create CI check for command registry synchronization
  - Add test that enumerates Commands enum variants
  - Verify each COMMANDS array entry has corresponding enum variant
  - Fail CI if COMMANDS contains non-existent commands
  - _Requirements: 9_

- [ ] 5.2 (P) Document command implementation checklist
  - Create checklist template in documentation
  - Include items: enum variant exists, implementation is complete not stub, behavioral tests verify correctness, manual testing completed, documentation updated with status
  - Add note that PLANNED commands must NOT appear in COMMANDS array
  - Reference checklist in contribution guidelines
  - _Requirements: 9_

- [ ] 5.3 (P) Document testing patterns for interactive commands
  - Create examples of piped stdin testing with std::process::Command
  - Document pattern for testing blocking behavior
  - Provide template for testing warning display to stderr
  - Include guidance on separating stdout and stderr in tests
  - _Requirements: 9_

### Phase 5: Exit Code and Error Handling Consistency

- [ ] 6. (P) Standardize exit codes and error messages
  - Verify all successful operations return exit code 0 including placeholder commands
  - Verify file errors and validation errors return exit code 2
  - Ensure invalid user input triggers re-prompt not program exit
  - Verify EOF on stdin results in graceful exit with code 0
  - Ensure all errors are written to stderr not stdout
  - Test that verbose mode displays implementation status warnings
  - _Requirements: 10_
