# Документация MakiseWE

Этот каталог связывает нормативную архитектуру, решения, evidence и acceptance scenarios. При конфликте действует следующий приоритет:

1. [INVARIANTS.md](../INVARIANTS.md);
2. принятые [ADR](adr);
3. [ARCHITECTURE.md](../ARCHITECTURE.md) и [PROTO.md](../PROTO.md);
4. domain specifications и roadmap;
5. historical/superseded documents.

## Рекомендуемый порядок чтения

1. [README.md](../README.md) — полный публичный обзор и quick start.
2. [VISION.md](../VISION.md) — назначение и release outcome.
3. [CONTEXT.md](../CONTEXT.md) — ubiquitous language.
4. [ARCHITECTURE.md](../ARCHITECTURE.md) — World Engine и causal boundaries.
5. [INVARIANTS.md](../INVARIANTS.md) — обязательные правила.
6. [WORLD_V1.md](../WORLD_V1.md) — мир, organisms и validation horizons.
7. [CIVILIZATION.md](../CIVILIZATION.md) — actions, devices, applications, institutions, services и construction.
8. [PROTO.md](../PROTO.md) — persistence, replay и migration.
9. [MEMORY.md](../MEMORY.md) — subjective memory каждого Consciousness.
10. [SECURITY.md](../SECURITY.md) — trust boundaries и disclosure policy.
11. [ROADMAP.md](../ROADMAP.md) — phases и gates.
12. [CHANGELOG.md](../CHANGELOG.md) — значимые изменения проекта.

## Evidence и scenarios

- [Phase 0 coverage matrix](coverage/phase0-coverage-matrix.md) фиксирует текущие contracts, evidence, unknowns и planned upgrades.
- [24-hour Human/Neko scenario](scenarios/phase1-24h-human-neko.md) задаёт первый runtime vertical slice до начала Phase 1.
- [Аудит биологической реалистичности](research/biology-realism.md) отделяет runtime evidence от целевой архитектуры и сверяет допущения с первичными источниками.
- [Contract schemas and fixtures](../contracts) являются machine-readable Phase 0 artifacts.

## Implementation plans

- [Минимальный V1 causal kernel](plans/0001-causal-kernel.md) — compatibility-safe последовательность для deep interface, canonical transitions, replay и одного thermal mechanism. Plan не является свидетельством реализованного runtime.

## Архитектурные решения

[ADR index](adr) перечисляет действующие, частично superseded и legacy-specific решения.

## История

[STAGE_5.md](../STAGE_5.md) сохранён для provenance прежнего плана. Он superseded текущей [ROADMAP.md](../ROADMAP.md) и не задаёт требования новой V1.

## Изменение документации

Изменение normative behavior обновляет связанный contract, ADR/invariant, fixtures, coverage matrix и public-seam tests. Правила contributions находятся в [CONTRIBUTING.md](../CONTRIBUTING.md).
