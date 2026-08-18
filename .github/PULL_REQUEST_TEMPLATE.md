## Outcome

Describe observable behavior and public seam.

## Scope

- Included:
- Explicit non-goals:
- Roadmap phase and gate:

## Causal and compatibility impact

Describe inputs/outputs, authoritative quantities and units, provenance, uncertainty, conservation, artifacts, replay, migration, and rollback or `SafeStop` behavior. Write “none” where not applicable.

## TDD evidence

- Red test and failure:
- Minimal green change:
- Independent expected-value source:

## Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Checklist

- [ ] Change uses project vocabulary and respects relevant ADR/invariants.
- [ ] Tests exercise public seams, not implementation details.
- [ ] Docs, schemas, fixtures and coverage matrix are updated where needed.
- [ ] No secrets, personal data, private conversations, runtime DB, or machine-specific paths are included.
- [ ] No arbitrary normalized authoritative score, hidden fidelity downgrade, morphotype-specific runtime branch, or direct LLM state mutation was added.
- [ ] Commit messages follow Conventional Commits.
