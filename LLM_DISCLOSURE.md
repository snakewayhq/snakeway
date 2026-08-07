# LLM Disclosure Report

**Date**: 2026-03-21 **Project**: Snakeway **LLM Used**: Claude (Anthropic) via Claude Code

## Overview

This document discloses how large language models were used during the development of Snakeway.
The analysis is based on code patterns, git history, commit attribution, and structural evidence within the codebase itself.

Snakeway is a human-designed, human-architected project.
LLMs were used as a force multiplier, primarily for systematic test generation, tedious refactoring tasks, and one major feature.
The core architecture, protocol integration, and design decisions are the product of human expertise.

## Summary of LLM Involvement by Area

| Area | LLM Involvement | Evidence |
|------|-----------------|----------|
| Config system | Minimal / None | Thoughtful two-layer design (Spec → Config) with strategic optimisations |
| Pingora integration | Minimal / None | Hook-ordering comments demonstrate first-hand source reading |
| Core architecture and design | Minimal / None | Deep domain expertise, strategic use of SmallVec, ArcSwap, error classification |
| ACME TLS feature | Collaborative | Co-author credit on 1 800-line feature commit and follow-up bug fix |
| Refactoring and cleanup | Collaborative | Magic-string consolidation, unwrap/expect audit, CI cleanup |
| Unit tests | Heavy | 100+ tests with formulaic AAA templates, Cartesian-product coverage |
| HTTP replay test framework | Heavy | 53 fixture files and 8 test modules created in a single commit |
| Integration test expansion | Heavy | Systematic coverage with RFC citations in every doc comment |

## Detailed Findings

### 1. Core Architecture, minimal to no LLM involvement

The foundational design shows deep Rust expertise and HTTP proxy domain knowledge that is not characteristic of LLM generation:

- **Two-layer config system** (`*Spec` for user-facing HCL, `*Config` for optimised runtime) is a deliberate architectural decision.
- **SmallVec usage** includes inline comments explaining *why* the optimisation was chosen, not just what it does.
- **Long-form architectural comments** in `traffic_proxy.rs` document Pingora hook execution order, which is knowledge that comes from reading upstream source code.
- **Error classification** (`error_classification.rs`) and domain-specific error types follow a consistent, intentional strategy across the codebase.
- **ArcSwap** for hot config reload is an expert-level concurrency choice.
- No `unsafe` bloat, no over-abstracted generics, no derive-heavy boilerplate.

### 2. ACME TLS feature, collaborative

Commit `0d5f50bc` ("Add Acme TLS automation support", ~1 800 lines) carries an explicit `Co-authored-by: Claude` attribution.
A follow-up bug fix (`fdaca76`) also credits Claude.
This is the most significant *feature* (as opposed to tests or refactoring) that involved LLM collaboration.

### 3. Refactoring and cleanup, collaborative

Several well-scoped, systematic tasks show LLM involvement:

- **Magic-string consolidation** (`b5bcde94`) extracted repeated string literals into a constants module across 8 files.
- **Unwrap/expect audit** (`b074c88b`) scanned the config-lowering path and replaced panic-able calls with proper error variants (73 insertions).
- **CI naming cleanup** (`c831e95a`) standardised GitHub Action names.
- **Documentation finalisation** (`4d919f57`) polished docs for the v0.9.0 release.

These are the kind of systematic, scan-and-fix tasks that LLMs are well suited to.

### 4. Unit tests, heavy LLM generation

LLM fingerprints are strongest in the unit test suite:

- **Formulaic AAA comments**, where every test follows an identical `// Arrange` / `// Act` / `// Assert` template with zero variation across 100+ tests.
- **Cartesian-product coverage**, where network filter tests mechanically enumerate every combination (IPv4 enabled or disabled, IPv6 enabled or disabled, and CIDR allow and deny precedence with IPs in each zone).
  A human developer would typically parameterise these.
  The LLM produced individual test functions.
- **Enable/disable test pairs**, where `test_ipv4_enabled` is immediately followed by `test_ipv4_disabled` with near-identical structure, repeated for every toggle.
- **Testing the standard library**, where some tests verify that `HeaderValue::from_bytes` rejects NUL bytes or that `HeaderName` auto-lowercases.
  These test the `http` crate, not Snakeway.

### 5. HTTP replay test framework, heavy LLM generation

Two commits on 2026-03-14, both authored by Claude, added the bulk of the HTTP replay test infrastructure:

- **`ccd7d8c`**, "add comprehensive HTTP replay test suite", expanded coverage from 9 tests to ~55 tests across 9 categories, creating 53 HTTP fixture files and associated test modules in a single commit.
- **`a43d7f6`**, "fill non-http-replay integration test gaps", added ~31 new tests across 7 new or extended test files.

Evidence of generation:

- **53 HTTP fixture files** created in one pass, all minimal, focused, and perfectly named by category (`methods/get_minimal.http`, `smuggling/cl_te.http`, `uri/null_byte_in_path.http`).
- **RFC citations in every test doc comment**, where `malformed.rs`, `methods.rs`, `uri.rs`, and `smuggling.rs` all cite specific RFC sections (RFC 9110 S5.3, RFC 9112 S3.2, etc.) in a formulaic pattern.
- **Structured commit messages** with bullet-point category breakdowns, in contrast to the developer's typically terse commit style.

### 6. Integration test expansion, heavy LLM generation

Broader integration test expansion exhibits the same patterns:

- **Dashed section dividers** (`//------- Category Name`) appear 12+ times with identical formatting.
- **Consistent test naming** following `test_[component]_[condition]_[expected_outcome]` across 141 tests.
- **Overly defensive assertions** that verify framework behaviour rather than application logic.

## Git History Evidence

| Signal | Detail |
|--------|--------|
| Explicit co-author credit | 6 commits carry `Co-authored-by: Claude <noreply@anthropic.com>` |
| Session URL references | Claude Code session URLs (`claude.ai/code/session_*`) appear in commit bodies |
| Collaboration window | LLM-attributed work clusters around 2026-03-02 to 2026-03-14 |
| Commit style contrast | LLM commits use structured bullet-point bodies, developer commits are terse |

## Conclusion

The developer uses Claude as a **tool for grunt work**, not as the architect.
The core proxy logic, configuration design, Pingora integration, and security model are human-authored.
LLMs were brought in for systematic test generation, codebase-wide refactoring, documentation polish, and one collaborative feature (ACME TLS).
All LLM contributions are transparently attributed in the git history.
