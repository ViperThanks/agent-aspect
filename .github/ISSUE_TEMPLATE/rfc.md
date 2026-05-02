---
name: RFC · Design Change Proposal
about: Propose changes to core design documents (PLAN.md, event model, policy matrix, etc.)
title: "[RFC] <brief description>"
labels: rfc
assignees: ''
---

> **Changes to the core design principles, event model, enforcement modes, or rule sources must go through this RFC process.**
> This applies to all documents under `docs/` that define the project's architecture.

## 1. Motivation (Why)

<!-- Why is this change needed? What scenario does the current design not cover? Use a concrete example. -->

## 2. Proposal (What)

<!-- What specifically changes. Reference section numbers from the relevant docs. -->

## 3. Design Principles Check (Required)

Does this proposal violate any of the following?

- [ ] **Exhaustive mechanisms** — Does it reduce the kernel's expressive power?
- [ ] **Gradual policy exposure** — Does it push complexity to users too early?
- [ ] **Traceable trust** — Does it weaken rule source attribution or the audit chain?

If any box is checked, explain below why the tradeoff is justified.

> Justification for breaking a principle:
>
> <!-- Fill in or remove this blockquote -->

## 4. Alternatives

<!-- List at least one "do nothing" alternative and why you rejected it. -->

## 5. Impact

- Affected documents:
  - [ ] `docs/PLAN.md` §<!-- section number -->
  - [ ] `docs/agent-capability-matrix.md`
  - [ ] `docs/unified-event-dto.md`
  - [ ] `docs/policy-matrix.md`
  - [ ] `docs/learn-mode.md`
- Affected phases:
  - [ ] Phase 1 (Mac core)
  - [ ] Phase 2 (iPhone + Watch)
  - [ ] Phase 3 (Ecosystem)

## 6. Migration

<!-- If the change affects existing code or data, describe the migration path. -->

## 7. Validation

<!-- How do we verify the change is correct? Walk through 2–3 real scenarios. -->

---

**Author checklist**:

- [ ] I have read the design principles in `docs/PLAN.md`
- [ ] I have read the original section I want to change
- [ ] I have considered at least one alternative
- [ ] If I broke a principle, I justified why
