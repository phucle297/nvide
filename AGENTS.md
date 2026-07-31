# AGENTS.md

# NVide AI Contributor Guide

This document defines how AI agents (Claude Code, Codex, GPT, Cursor, etc.) must work on NVide.

Failure to follow this guide is considered a bug.

---

# Priority Order

When multiple documents disagree, follow this order.

1. User prompt
2. Accepted ADR
3. Architecture Document
4. Phase Roadmap
5. Coding Standards
6. Existing implementation

Never violate a higher-priority document.

---

# Repository First

Before writing any code, the AI MUST understand the existing project.

Read, at minimum:

- relevant Architecture documents
- affected ADRs
- related modules
- existing tests
- public APIs used by the target module

Do not begin implementation before understanding the current design.

---

# Search Before Creating

Before creating any new

- file
- module
- type
- trait
- interface
- utility
- helper
- service

search the repository first.

Prefer reusing existing implementations.

Avoid duplicate logic.

---

# Before Writing Any Code

The AI MUST identify

- current implementation phase
- target milestone
- affected ADR(s)
- expected deliverables

If any are unclear,

STOP

Ask for clarification.

Never guess.

Before implementation, produce

```text
## Context

Phase:
Milestone:
Affected ADR:
Deliverables:

## Requirement Checklist

- [ ]
- [ ]

## Implementation Plan

1.
2.
3.
```

The checklist must be derived only from

- user request
- ADR
- Architecture
- Roadmap

Do not invent additional requirements.

---

# Scope Rules

Only implement what belongs to the current phase.

Never implement work from later phases.

Example

Phase 0

Allowed

- workspace
- CI
- rope buffer
- undo
- IPC prototype
- renderer prototype
- architecture docs
- ADRs

Not allowed

- plugins
- debugger
- LSP
- Git integration
- marketplace
- terminal
- remote development

---

# Requirement Checklist

Before implementation,

generate a requirement checklist.

After implementation,

verify every requirement.

Do not mark complete unless implemented.

---

# Architecture First

Never invent architecture.

Use existing ADRs.

If implementation requires architectural changes,

DO NOT change it silently.

Instead

1. explain the conflict
2. propose an ADR update
3. wait for approval

---

# ADR Compliance

Every implementation should reference affected ADRs.

Example

Implements

- ADR-0002
- ADR-0005

If no ADR exists,

ask whether a new ADR should be created.

---

# Repository Respect

Prefer consistency over personal preference.

Follow the existing

- architecture
- naming
- module layout
- coding patterns
- error handling style

Do not rewrite code simply because another approach appears cleaner.

Improve only when explicitly requested or required.

---

# No Hidden Features

Do not add

- nice-to-have functionality
- future-proof abstractions
- extension points
- optional features
- generic frameworks

unless explicitly requested.

Examples

Wrong

Implement RopeBuffer

and also

- generic backend
- CRDT hooks
- plugin support

Correct

Implement RopeBuffer only.

---

# Avoid Over Engineering

Choose the simplest implementation that satisfies the current phase.

Future phases may redesign internals.

Do not optimize prematurely.

Avoid introducing

- unnecessary abstractions
- excessive generics
- additional layers
- speculative extension points

---

# API Rules

Never invent

- APIs
- traits
- interfaces
- module names
- file names
- commands
- configuration formats

Search the repository first.

Reuse existing conventions.

If uncertain,

ask.

---

# Editing Rules

Prefer the smallest possible change.

Only modify files required for the task.

Avoid

- unrelated refactoring
- formatting unrelated code
- renaming unrelated symbols
- moving files
- changing import order globally

unless explicitly requested.

---

# Refactoring Rules

Never refactor unrelated code.

Allowed

- compile fixes
- bug fixes
- formatting in modified code
- naming consistency
- dead code directly related to the task

Not allowed

- large rewrites
- architecture changes
- cleanup outside requested scope

---

# Error Handling

Follow the project's existing error handling strategy.

Do not introduce

- unwrap()
- expect()
- panic!()
- TODO placeholders
- unimplemented!()

unless

- existing code already follows that pattern
- explicitly requested
- writing tests

---

# Documentation

If public APIs change,

update

- Architecture (if needed)
- ADR (if needed)
- Coding Standards (if affected)
- Rust documentation
- examples (if affected)

Documentation should be updated in the same task.

---

# Testing Rules

Every feature must include appropriate tests.

Priority

1. unit tests
2. property tests
3. integration tests

Tests should verify

- expected behavior
- edge cases
- regressions
- error handling

Do not add expensive benchmarks unless requested.

---

# Completion Criteria

A task is NOT complete unless

- all requirements implemented
- tests pass
- documentation updated
- no scope violations
- no unrelated changes
- builds successfully

---

# Pull Request Checklist

The AI must verify

- [ ] matches current phase
- [ ] matches milestone
- [ ] satisfies deliverables
- [ ] follows ADR
- [ ] no hidden features
- [ ] no duplicate implementation
- [ ] tests added
- [ ] documentation updated
- [ ] builds successfully
- [ ] no unrelated changes

---

# Code Review Rules

When reviewing code,

evaluate

1. Requirement coverage
2. Scope compliance
3. ADR compliance
4. Architecture quality
5. Simplicity
6. Maintainability
7. Performance
8. Testing quality
9. Error handling
10. Duplicate implementation
11. Future migration risk
12. Scope creep

Do not review only code style.

Always compare implementation against

- current phase
- roadmap
- ADR
- architecture

---

# Decision Policy

When uncertain,

Never assume.

Explain available options.

Wait for user confirmation.

---

# Golden Rules

Correctness > Completeness

Architecture > Convenience

Current Phase > Future Phase

Consistency > Personal Preference

Simple > Clever

Reuse > Reinvent

Small Changes > Large Refactors

When uncertain, ask.
