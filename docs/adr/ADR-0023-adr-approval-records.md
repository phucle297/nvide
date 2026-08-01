# ADR-0023: ADR approval records

- Status: Accepted
- Date: 2026-07-31
- Extends: Architecture v0.2.1 ADR process
- Prepared-by principals: `agent:/root`

## Context

The Architecture defines ADR statuses, storage, sections, acceptance roles, and reviewer independence but needs a durable approval-record format, amendment record, and RFC quorum rule. P0.1 cannot claim approval without inspectable evidence.

## Decision

The canonical workflow remains `docs/architecture.html#adr-process`; no duplicate workflow index is created.

Each canonical ADR identifies every author or implementer principal and contains an approval table with role, reviewer principal, verdict, UTC date, reviewed commit, and inspectable artifact link. The artifact itself identifies the same reviewer principal, verdict, reviewed commit, and review scope. Principals use `human:<handle>` or `agent:<orchestrator-issued-canonical-task-id>`.

Human and AI reviews both count. A review is independent only when its reviewer principal differs from every author or implementer principal and every other required reviewer principal. For AI, a later turn by the same agent is still the same reviewer. One agent cannot author or implement the change and approve it, or fill multiple required reviewer slots.

`Proposed` changes to `Accepted` only after the Architecture acceptance rule is met. Migrated catalog decisions may retain `Accepted`, but they do not satisfy P0.1 until their historical approval or a revalidation record is attached.

A material change to an Accepted decision creates a new monotonic ADR and marks the old record `Superseded by ADR-XXXX`. A non-material clarification appends a dated amendment and repeats the normal approval rule.

P0.1 uses tech lead plus one independent reviewer. Cross-cutting RFC voting is unavailable until another Accepted process defines quorum, eligibility, voting period, and durable vote records.

## Alternatives

- A separate ADR workflow file would duplicate the Architecture process.
- Free-form approval prose is rejected because it cannot be checked consistently.
- Rejecting all AI review is unnecessary; identity separation and inspectable evidence prevent self-approval without requiring a human-only process.

## Pros/Cons

Pros: inspectable approval, unambiguous reviewed revision, usable human or AI reviewers, and no duplicated workflow source.

Cons: migrated catalog decisions require revalidation when historical records are unavailable, and agent principals must be recorded consistently.

## Consequences

ADR-0002, ADR-0003, and ADR-0005 remain P0.1 blockers until their pending approval records are completed. Proposed ADR-0020–ADR-0023 require normal acceptance before their decisions can drive implementation. AI agents may supply those reviews when the distinct-principal rule is satisfied.

## Migration

Add prepared-by principals and the standard table to existing ADRs when they are next reviewed. Do not fabricate historic reviewer identities or dates.

## Scalability

The fixed principal syntax and fields support later automated uniqueness and self-review checks without a separate approval database.

## Approval record

| Role | Reviewer principal | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | `agent:/root/ai_review_policy` | AGREE | 2026-08-01 | `710644906cd9589c2e3f2c25a8484088e710feac` | [`7106449-p0-plan-policy.md`](../reviews/7106449-p0-plan-policy.md) |
| Independent reviewer | `agent:/root/ai_review_records` | AGREE | 2026-08-01 | `710644906cd9589c2e3f2c25a8484088e710feac` | [`7106449-p0-records.md`](../reviews/7106449-p0-records.md) |
