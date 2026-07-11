# CODING.md (Rust Coding Policy)

## 1. Clippy Compliance

* **Zero Warnings**: Every modification must compile and pass `cargo clippy --all-targets` under strict verification settings.
* **Pedantic Quality**: Code must strictly conform to high-quality standards.
* **No Unjustified Bypassing**: NEVER use `#![allow(...)]` or `#[allow(...)]` to suppress compiler warnings or lint errors unless it is mathematically impossible to refactor the code to comply. Any allowance must be justified in a comment.

---

## 2. Size, Complexity & Fragmentation Safeguards

* **File Length Limit**: Max 300 lines per file.
* *Exception*: Files dedicated entirely to large AST (Abstract Syntax Tree) definitions, exhaustive central `enum` definitions, or central MCP Trait definitions are permitted to expand up to 500 lines to preserve logical cohesion.


* **Function Length Limit**: Max 40 lines per function.
* *SRP Invariant*: Every function must adhere strictly to the Single Responsibility Principle.
* *Anti-Fragmentation Rule*: Do NOT aggressively fragment large `match` statement branches or sequential error handling pipelines into artificial sub-functions purely to satisfy the line count. Prioritize logical cohesion and readability over mechanical line-splitting.


* **Strict Test Separation**: The production code space must remain entirely free of testing infrastructure.
* NEVER embed `#[cfg(test)]` modules directly within a production file.
* All unit and integration tests must reside in separate companion test files (e.g., `src/module/tests.rs`). In the parent module, declare the test module using `#[cfg(test)] mod tests;`. Test files must also respect the 300-line length constraint.



---

## 3. Implementation Scope & Invariants

* **Strict Task Adherence**: Implement **only** the types, functions, and logic strictly demanded by the current active Atomic Work Unit (AWU).
* **No Speculative Engineering**: Do NOT introduce speculative traits, unused generics, dead helper functions, or "future-proofing" frameworks. Write code exclusively for the present requirement.
* **No Code Placeholders**: Empty function stubs, dummy return values, or temporary `TODO` / `FIXME` comments are strictly prohibited in the final implementation phase. Every line of code must be fully operational before hitting the Audit Phase.

---

## 4. Safety & Error Handling Paradigms

* **Domain-Specific Errors**: Leverage the `thiserror` crate to model domain-specific, precise, and strongly typed error structures in low-level library/core modules.
* **Application Error Propagation**: Use the `anyhow` crate for high-level application entry points, CLI wrappers, or top-level MCP logic where universal error context propagation is required.
* **Zero Panics**: The functions `unwrap()` and `expect()` are strictly banned from production code. Every potential failure path must be handled explicitly via idiomatic `Result` or `Option` propagation. These panic-inducing functions are only permitted within isolated test companion files.
* **Memory Safety**: The `unsafe` keyword is completely prohibited in the workspace. All logic must satisfy the compiler's safe Rust invariants.
* **Lifetime Management**: Avoid complex, deeply nested explicit lifetime annotations. Favor clean ownership transfer, intentional object cloning where performance characteristics permit, or standard smart pointers (`Rc` / `Arc`).

---

## 5. Localized Footprint & Precision

* **Minimal Modifications**: Solve every programming problem with the smallest possible footprint. Keep code changes surgical and tightly localized.
* **No Collateral Style Changes**: Do NOT refactor or reformat adjacent, unrelated code, variables, or styling conventions. Keep Git diffs highly focused on the specific task to enable seamless auditing.

---

## 6. Security Boundaries

* **No Absolute Paths**: Never bake absolute hardcoded filesystem paths into code or configuration lookups. Always resolve paths relative to the runtime root or discover them dynamically via environment variables verified by the MCP layer.
* **Zero Hardcoded Secrets**: Hardcoded API tokens, cryptographic keys, credentials, or private addresses are strictly banned. All configuration parameters must be fetched at runtime via standard environment variables.
