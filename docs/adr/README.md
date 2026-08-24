# Architecture Decision Records

ADR фиксируют труднообратимые решения и причины. Новый ADR не переписывает историю: прежний документ получает явный `superseded` status и ссылку на замену.

## Index

- [ADR-0001: отдельная система Makise](0001-separate-system.md) — accepted isolation boundary.
- [ADR-0002: World Engine — единственный автор истины](0002-world-authority.md) — accepted authority rule; command-shaped API superseded deep-module `commit` contract.
- [ADR-0003: реальное время и durable activities](0003-real-time-and-durable-activities.md) — partially superseded by ADR-0009.
- [ADR-0004: WorldService через bounded actor и UDS](0004-world-service-uds.md) — accepted legacy runtime transport; future API remains adapter-only.
- [ADR-0005: stable causal interfaces](0005-stable-causal-interfaces.md) — accepted.
- [ADR-0006: unitful authoritative state](0006-unitful-authoritative-state.md) — accepted.
- [ADR-0007: independent data-driven morphotypes](0007-independent-morphotypes.md) — accepted.
- [ADR-0008: cognitive acceptance pipeline](0008-cognitive-acceptance-pipeline.md) — accepted.
- [ADR-0009: canonical simulation time](0009-canonical-simulation-time.md) — accepted.
- [ADR-0010: content-addressed artifacts](0010-content-addressed-artifacts.md) — accepted.
- [ADR-0011: unified causal graph](0011-unified-causal-graph.md) — partially superseded by ADR-0013; основа единого graph и timeline сохранена, карта расширена L8–L9.
- [ADR-0012: intentions запускают causal processes](0012-causal-processes-not-promised-outcomes.md) — accepted; semantic actions не обещают outcome.
- [ADR-0013: diegetic technology и institutions](0013-diegetic-technology-and-institutions.md) — accepted; software, services и construction остаются внутри causal world.
- [ADR-0014: fidelity envelope и validation evidence](0014-fidelity-envelope-and-validation-evidence.md) — accepted; realism claims требуют provenance, uncertainty и validation horizon.

Normative invariants находятся в [INVARIANTS.md](../../INVARIANTS.md); module boundary — в [ARCHITECTURE.md](../../ARCHITECTURE.md).
