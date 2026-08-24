# Changelog

Все значимые изменения MakiseWE фиксируются в этом файле. Формат основан на [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning следует [Semantic Versioning](https://semver.org/spec/v2.0.0.html) после появления первого release tag.

## [Unreleased]

### Added

- Phase 0 contract schemas и fixtures для mechanisms, resolutions, morphotypes и cognitive decisions.
- Independent Human/Neko root morphotype examples.
- Cell-cohort и neural-population resolution round-trip evidence.
- Public documentation index, contribution guide, Code of Conduct, issue/PR templates, Dependabot и CI.
- Нормативная causal civilization specification для closed-loop actions, техники, приложений, organizations, services, экономики и construction.
- ADR о запрете promised outcomes и ADR о diegetic technology/institutions с capability-mediated external effects.

### Changed

- Архитектура V1 стала многомасштабной и contract-driven.
- Simulation закреплена как единый causal graph с cross-cutting durable timeline и explicit causally triggered resolution transitions.
- Causal map расширена digital/computation и institutional/economic domains L8–L9 без превращения domains в pipeline.
- Semantic actions, contracts, designs и application requests закреплены как intentions/causes, а не прямые outcome mutations.
- Model improvement вынесен во внешний validated control plane; autonomous production activation исключена из V1.
- World Engine design получил единый `commit` mutation boundary.
- LLM role ограничена `CortexProposal`; adoption требует `CognitiveDisposition::Accepted`.
- README, memory и security design согласованы с Phase 0 architecture.
- Старый Stage 5 plan помечен superseded и сохранён как history.

### Existing legacy-compatible core

- Rust single-writer world state, SQLite event log, snapshots и deterministic replay.
- Protobuf/gRPC WorldService через Unix Domain Socket и C++ WorldClient.
- Data-defined apartment packages, durable activities, environment projections и recovery tests.

Первый release ещё не опубликован. Содержимое `Unreleased` не является стабильным API promise.
