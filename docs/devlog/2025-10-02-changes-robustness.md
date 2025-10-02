# 2025-10-02 – Robustness Enhancements

## Summary
- Switched CLI action parsing to an `enum Action` with custom parsers for both action and speed, enforcing valid input and clamping speed within `1..=1000` ms.
- Added a `TerminalGuard` utility to centralize alternate-screen/raw-mode lifecycle management and guarantee cleanup via `Drop`.
- Updated `typewriter_print()` and `tui_reveal()` to reuse the guard, support early exit keys (Esc, q, Enter, Ctrl+C), and respect resize events.
- Improved poem lookup by trimming input, standardizing case-insensitive search, sorting listings, and rejecting empty names.
- Introduced unit tests covering action parsing, speed bounds, case-insensitive poem retrieval, and empty-name validation.

## Motivation
Prior implementation relied on stringly-typed action branching, lacked bounds checking for speed, and risked leaving the terminal in a bad state if the program exited early. The refinements ensure safer input handling, resilient terminal teardown, and verifiable behavior through tests.
