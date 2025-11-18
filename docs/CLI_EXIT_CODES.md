# CLI Exit Code Standards

## Overview

This document defines the standardized exit codes and error handling behavior for the Axiomind CLI (`axiomind`). All commands must follow these conventions to ensure consistent user experience.

## Exit Codes

### Standard Exit Codes

| Code | Meaning | When Used | Examples |
|------|---------|-----------|----------|
| `0` | Success | Command completed successfully (including placeholder commands) | Successful play, sim, deal, bench, cfg, doctor (when all checks pass) |
| `2` | Error | File errors, validation errors, system errors | Missing input file, invalid parameters, parse errors, IO failures |
| `130` | Interrupted | User terminated with Ctrl+C | Interrupted simulation, interrupted play session |

### Exit Code Policy

1. **Success (0)**: Return `0` for any command that completes its intended operation successfully
   - This includes placeholder/demo commands that display warnings but execute
   - Example: `axiomind eval` returns `0` even though it's a placeholder (with prominent warnings)

2. **Error (2)**: Return `2` for any error condition that prevents command execution
   - File not found or read errors
   - Invalid parameter values (e.g., `--hands 0`)
   - Validation failures (e.g., splits don't sum to 100%)
   - Parse errors in input files
   - IO failures during write operations

3. **Interrupted (130)**: Return `130` when user interrupts with Ctrl+C
   - Allows scripts to distinguish user interruption from errors
   - Handled by signal handler in main binary

## Error Message Standards

### Error Output Location

**All errors MUST be written to stderr, never stdout.**

```rust
// Correct: Error to stderr
let _ = ui::write_error(err, "hands must be >= 1");

// Incorrect: Error to stdout
println!("Error: hands must be >= 1");
```

### Warning Format

Warnings for placeholder/demo implementations must follow this format:

```
WARNING: <clear description of limitation>
```

Example:
```
WARNING: AI opponent is a placeholder that always checks. Use for demo purposes only.
```

### Error Message Style

- **Be specific**: "hands must be >= 1" not "invalid parameter"
- **Include context**: "Failed to read /path/to/file.jsonl: No such file or directory"
- **Suggest fixes**: "Splits must sum to 100% (1.0 total)"
- **Consistent terminology**: Use "Failed to read", "Invalid", "Missing" consistently

## Command-Specific Behavior

### Interactive Commands (play --vs human)

**Invalid User Input**: Do NOT exit the program
- Invalid action input should trigger error message and re-prompt
- Only EOF or quit command should terminate the session
- Exit code should be `0` for both graceful quit and EOF

```rust
match parse_player_action(&input) {
    ParseResult::Invalid(msg) => {
        let _ = ui::write_error(err, &msg);
        // Re-prompt, don't return error code
    }
    ParseResult::Quit => {
        // Exit gracefully with code 0
        return 0;
    }
    // ...
}
```

### Placeholder Commands

Placeholder commands (e.g., `eval`, `play --vs ai`) should:
1. Display prominent warning to stderr before execution
2. Complete their placeholder operation
3. Return exit code `0` (success)
4. Tag output to indicate demo/placeholder status

This allows scripts to continue running while making limitations clear to users.

## Implementation Checklist

When implementing or modifying a command:

- [ ] Successful execution returns `0`
- [ ] File errors return `2`
- [ ] Validation errors return `2`
- [ ] All errors written to stderr (never stdout)
- [ ] Invalid user input triggers re-prompt (interactive commands only)
- [ ] EOF handled gracefully with exit code `0`
- [ ] Placeholder commands display "WARNING:" prefix
- [ ] Error messages are descriptive and actionable

## Testing Requirements

Every command MUST have tests verifying:

1. **Success case returns 0**
   ```rust
   #[test]
   fn test_<command>_success_returns_zero() {
       let code = axiomind_cli::run(args, &mut out, &mut err);
       assert_eq!(code, 0);
   }
   ```

2. **Error cases return 2**
   ```rust
   #[test]
   fn test_<command>_error_returns_two() {
       let code = axiomind_cli::run(invalid_args, &mut out, &mut err);
       assert_eq!(code, 2);
   }
   ```

3. **Errors go to stderr**
   ```rust
   #[test]
   fn test_<command>_errors_to_stderr() {
       let code = axiomind_cli::run(invalid_args, &mut out, &mut err);
       assert_eq!(code, 2);
       assert!(String::from_utf8_lossy(&err).contains("expected error"));
       assert!(!String::from_utf8_lossy(&out).contains("expected error"));
   }
   ```

## Examples

### Successful Command
```bash
$ axiomind deal --seed 42
# Output to stdout
# Exit code: 0
```

### Validation Error
```bash
$ axiomind play --vs ai --hands 0
# Error to stderr: "hands must be >= 1"
# Exit code: 2
```

### File Error
```bash
$ axiomind replay --input /nonexistent.jsonl
# Error to stderr: "Failed to read /nonexistent.jsonl: No such file or directory"
# Exit code: 2
```

### Placeholder Warning
```bash
$ axiomind play --vs ai --hands 1
# Warning to stderr: "WARNING: AI opponent is a placeholder that always checks. Use for demo purposes only."
# Output to stdout with [DEMO MODE] tags
# Exit code: 0
```

### Interactive Invalid Input
```bash
$ axiomind play --vs human --hands 1
Enter action: invalid
# Error to stderr: "Unrecognized action 'invalid'. Valid actions: fold, check, call, bet <amount>, raise <amount>, q"
Enter action: fold
# Continues execution, exit code: 0 when complete
```

### Graceful Quit
```bash
$ axiomind play --vs human --hands 10
Enter action: q
# Session summary to stdout
# Exit code: 0
```

## Backward Compatibility

These standards codify existing behavior. No breaking changes are introduced:
- Commands that returned `0` continue to return `0`
- Commands that returned `2` continue to return `2`
- Error output to stderr is preserved
- Placeholder commands maintain exit code `0` for script compatibility

## References

- [CLI Implementation Status](./CLI_IMPLEMENTATION_STATUS.md)
- [Exit code tests](../rust/cli/tests/test_exit_codes.rs)
- Requirement 10: CLI User Experience Consistency (`.kiro/specs/cli-audit-fixes/requirements.md`)

## Revision History

- 2025-11-13: Initial documentation of exit code standards (Task 6 implementation)
