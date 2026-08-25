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
- Phase 2 causal-kernel физические механизмы: метрическое rigid body (3D pose, center-of-mass, principal inertia) с точной консервацией энергии; box-контакты и grasp friction cone с projection удержания; data-driven articulated skeleton из anatomy graph морфотипов с joint limits и torque port; детерминированные physics islands с worker parity, явными suspend/resume transitions и физическим rest trigger через support; collision response с точной консервацией импульса, restitution fixed point и тангенциальным Coulomb friction (stick/slide через целочисленный friction cone); bipedal balance assessment по point/segment опоре; durable walk `ControlEpisode` с balance feedback, blockers и replanning без promised completion; fluid statics с measured плотностью воды (ADR-0014) — гидростатика, плавучесть, точный flotation verdict; pour/spill учёт объёма с bit-exact conservation, rim overflow как первоклассным исходом и наблюдаемой глубиной лужи; room-atmosphere Compartment с measured плотностью/теплоёмкостью сухого воздуха (ADR-0014), конвективной проводимостью G = h·A через существующий thermal port, конвертацией мощности нагревателя, точным liquid↔vapour массовым мостом и проекцией абсолютной влажности; point-source распространение полей с declared inverse-square затуханием: звук (fW/m², measured порог слышимости [ISO 226]), свет (π-свободный фотометрический закон E = I_v/d² от кандел), запахи как expert_estimate surrogate неподвижного воздуха — каждая модальность со своей validity band и free-field предположением; electricity/water сети как unit-typed flow-conservation механизмы с admission против declared capacity, точной доставкой E = P·t и V = rate·t под cumulative metering и typed остановкой при отключении без promised outcome; multi-step cook/clean/dress `ControlEpisode`s поверх перечисленных механизмов с duration, возникающей из физики, наблюдаемым completion, durable blockers и partial results как первичными исходами; durable body records — `CommitRequest::place_body` проводит named metric rigid bodies через единственный mutation path с идемпотентным retry, typed конфликтами и bit-exact восстановлением после reopen.

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
