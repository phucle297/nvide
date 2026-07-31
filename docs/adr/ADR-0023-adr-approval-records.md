# ADR-0023: ADR approval records

- Status: Proposed
- Date: 2026-07-31
- Extends: Architecture v0.2.0 ADR process

## Context

The Architecture defines ADR statuses, storage, sections, and acceptance roles but not a durable approval-record format, self-review rule, amendment record, or RFC quorum. P0.1 cannot claim approval without inspectable evidence.

## Decision

The canonical workflow remains `docs/architecture.html#adr-process`; no duplicate workflow index is created.

Each canonical ADR contains an approval table with reviewer name/handle, role, verdict, UTC date, reviewed commit, and inspectable artifact link. Self-review and AI quality review do not count as independent approval.

`Proposed` changes to `Accepted` only after the Architecture acceptance rule is met. Migrated catalog decisions may retain `Accepted`, but they do not satisfy P0.1 until their historical approval or a revalidation record is attached.

A material change to an Accepted decision creates a new monotonic ADR and marks the old record `Superseded by ADR-XXXX`. A non-material clarification appends a dated amendment and repeats the normal approval rule.

P0.1 uses tech lead plus one independent reviewer. Cross-cutting RFC voting is unavailable until another Accepted process defines quorum, eligibility, voting period, and durable vote records.

## Alternatives

- A separate ADR workflow file would duplicate the Architecture process.
- Free-form approval prose is rejected because it cannot be checked consistently.
- Treating AI review as approval is rejected because it creates false external evidence.

## Pros/Cons

Pros: inspectable approval, unambiguous reviewed revision, and no duplicated workflow source.

Cons: migrated catalog decisions require revalidation when historical records are unavailable.

## Consequences

ADR-0002, ADR-0003, and ADR-0005 remain P0.1 blockers until their pending approval-record attachments are replaced by inspectable records. Proposed ADR-0020–ADR-0023 require normal acceptance before their decisions can drive implementation.

## Migration

Add the standard table to existing ADRs when they are next reviewed. Do not fabricate historic reviewer identities or dates.

## Scalability

The fixed fields support later automated checks without a separate approval database.

## Approval record

| Role | Reviewer | Verdict | UTC date | Reviewed commit | Artifact |
| --- | --- | --- | --- | --- | --- |
| Tech lead | PENDING | PENDING | PENDING | PENDING | PENDING |
| Independent reviewer | PENDING | PENDING | PENDING | PENDING | PENDING |
