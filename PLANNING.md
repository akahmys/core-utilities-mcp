# PLANNING.md (Agent Planning Policy)

## 1. Core Directives

* **AWU-Driven Execution**: Decompose all user requests and long-term milestones into Atomic Work Units (AWUs) that are fully completable in a single execution turn or cycle.
* **Centralized Project State**: Maintain `PLANS.md` as the absolute, single source of truth for all project states, progress tracking, and execution histories.
* **Plan Integrity**: NEVER delete, omit, or overwrite existing historical logs, completed roadmap items, or pending AWUs in `PLANS.md` unless explicitly instructed by the user. Historical transparency must be preserved.

---

## 2. AWU (Atomic Work Unit) Criteria

Every short-term plan item must be broken down into an AWU that satisfies ALL of the following conditions:

* **Clear Definition of Done (DoD)**: Exactly one objective, unambiguous and measurable (e.g., "All unit tests pass," "Compilation succeeds with zero warnings").
* **Tight & Localized Scope**: A strictly limited target file set. The modification must focus on minimizing collateral changes (ideally impacting $\le$ 3 files per unit).
* **Context Window Fit**: Optimized for token efficiency. Avoid designing massive AWUs that force the agent to inspect large portions of the codebase simultaneously. Leverage `mcp-uutils` to keep file analysis granular.

---

## 3. Planning Layers

### Long-Term Plan (Roadmap)

* Represents high-level engineering phases and macro-milestones.
* Keep text minimal and structural (e.g., Phase 1, Phase 2). Do not write granular code implementation details here.

### Short-Term Plan (Next 3–5 AWUs)

* Represents the immediate next steps derived from the current active roadmap phase.
* Expand roadmap items into detailed AWUs dynamically on-demand (only when the current AWU is nearing successful completion or during initial planning steps).
* Explicitly define the **Objective**, **Scope**, and **DoD** for each unit.

---

## 4. Plan Revision Protocol

If the implementation phase reveals a fundamental design flaw, an unexpected blocking issue, or clear scope creep:

1. **STOP** execution immediately. Do not guess or proceed blindly with a broken plan.
2. **Propose Revision**: Formulate a concise description of the engineering discrepancy and propose the necessary updates to the short-term or long-term plan.
3. **Wait for User Approval**: Do not modify files or proceed with the revised workflow until the user explicitly approves the shift.
4. **Log the Transition**: Once approved, update `PLANS.md` accordingly and document the exact reason for the mid-course correction in the previous AWU's `Result` field.

---

## 5. `PLANS.md` Structural Template

All updates to `PLANS.md` made via `mcp-uutils` must strictly follow this visual structure:

```markdown
# Project Work Plan (PLANS.md)
**Last Updated**: YYYY-MM-DD

## 🗺️ Long-Term Plan (Roadmap)
- [✅] Phase 1: Core Foundation & Infrastructure Setup
- [🔄] Phase 2: Implementation of Core Subsystems
- [ ] Phase 3: Integration Tests & Polishing

---

## 🛠️ Short-Term Plan: Phase 2 (Core Subsystems)

### 💡 Current AWU Status
- [✅] AWU-1: [Completed] Define internal data structures and baseline traits.
- [🔄] AWU-2: [In Progress] Implement JSON serialization layer.
- [ ] AWU-3: [Todo] Add comprehensive integration verification hooks.

### 📝 AWU Details

#### AWU-1: Define internal data structures and baseline traits.
- **Objective**: Establish shared core types for the system.
- **Scope**: `src/types.rs`, `src/traits.rs`
- **DoD**: `cargo check` passes with zero warnings.
- **Result**: Successfully implemented. Structures match design constraints.

#### AWU-2: Implement JSON serialization layer.
- **Objective**: Connect core types to standardized JSON endpoints.
- **Scope**: `src/json.rs`, `src/types.rs`
- **DoD**: All serialization unit tests pass in `src/json/tests.rs`.

```
