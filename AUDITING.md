# AUDITING.md (Audit Rules & Checklist)

**Trigger**: Completion of every active Atomic Work Unit (AWU) $\rightarrow$ Perform full Audit Phase $\rightarrow$ Update `PLANS.md` upon comprehensive verification.

---

## 📋 1. Planning & State Audit

* **Roadmap & History Integrity**: Ensure the entire visual structure of `PLANS.md` is fully preserved. Past completion logs, chronological records, and untouched roadmap items must never be deleted, truncated, or omitted.
* **Atomicity Check**: Verify that the completed work represents exactly one discrete, atomic unit of work matching the target Definition of Done (DoD). No loose ends or partial implementations.
* **Log Consistency**: Verify that the `Result` field in `PLANS.md` accurately documents the real-world outcome, including any approved mid-course design adjustments or tool constraints encountered.

---

## ⚡ 2. Token & Resource Efficiency Audit

* **On-Demand Inspection**: Confirm that no files outside the immediate scope of the AWU were parsed or loaded unnecessarily. Verify that line-windowed tools (`peek_file`) were utilized to scan heavy assets instead of bulk reading.
* **Incremental Modifications**: Review the implementation footprint. Ensure that changes were applied surgically (e.g., via `file_edit_patch`) rather than performing aggressive, full-file rewrites that pollute Git diff history.
* **Context Hygiene**: Verify that verbose compiler errors, multi-page test dumps, or raw output streams have been completely pruned from the persistent reasoning state to maximize available token limits.

---

## ⚖️ 3. Policy & Compliance Audit

* **Size Invariants**: Check file and function lengths to ensure strict alignment with architectural standards:
* Production source files must remain under 300 lines (or under 500 lines exclusively for approved central traits, AST definitions, or core enums).
* Functions must remain under 40 lines while preserving structural cohesion (no artificial splitting of logical `match` pipelines).


* **Test Infrastructure Separation**: Verify that no `#[cfg(test)]` block or test code has bled into production files. All test scenarios must be strictly isolated within separate companion test files.
* **Scope Creep Invariant**: Confirm that no speculative code, unused helper methods, dummy functions, or temporary placeholders (`TODO` / `FIXME`) exist in the final codebase.

---

## 🔍 4. Documentation Audit

* **API Documentation**: Ensure that all new or altered public structs, traits, enums, and functions are paired with exhaustive, well-structured Rust documentation comments (`///`).
* **Real-world Sync**: Verify that all documentation text accurately mirrors the actual, present state of the implementation logic. Speculative doc descriptions are prohibited.

---

## 🤖 5. Mechanical Audit (Automated Verification)

Execute the following verification suite via `mcp-uutils:spawn_bash_process` and ensure all checks achieve flawless termination:

* **Compilation**: `cargo check` passes with absolute zero errors.
* **Formatting**: `cargo fmt --check` succeeds with no style discrepancies.
* **Linting**: `cargo clippy --all-targets` passes with **zero warnings** under strict `#![deny(clippy::pedantic)]` constraints.
* **Testing**: `cargo test` successfully clears all unit, integration, and doc-test suites across target workspaces.
* **State Audit**: Execute `mcp-uutils:verify_file` on altered assets to double-check exact file metrics (lines, bytes, and hashes) before finalized logging.

---

## ⚠️ Violation Protocol

**If any quality audit criterion fails, or if a structural regression is detected:**

1. **STOP** execution loops immediately. Do not attempt to log the unit as completed or move on to a subsequent task.
2. **Identify**: Isolate the exact broken rule, lint warning, or failing test case.
3. **Remediate**: Surgically adjust the implementation code or re-align the short-term plan to correct the breakdown.
4. **Re-Audit**: Re-trigger the complete `AUDITING.md` verification cycle from step 1.
5. **Report**: Formulate a clear explanation for the user detailing the exact compilation or policy failure, the specific remediation applied, and the successful outcome of the final re-audit.
