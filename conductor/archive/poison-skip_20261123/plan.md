# Plan

- [x] Task 1: test red (E0438 no `DEAD_LETTERS_POISON`), the const (default true) and `poison` skipping when false; 10 passed, CI green 27b27a2
- [x] Review: no findings (the default keeps every existing consumer's behaviour; a failing handler is dead-lettered either way, tested)
