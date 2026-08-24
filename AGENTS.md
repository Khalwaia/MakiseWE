# Инструкции для агентов MakiseWE

## Миссия и текущая граница

MakiseWE — персистентная многомасштабная причинная симуляция. Цель разработки — проверяемая причинность внутри объявленного `FidelityEnvelope`, а не правдоподобный сценарий без измеримых оснований.

Phase 0 зафиксировал нормативную архитектуру. Существующий `makise-world` остаётся legacy-compatible runtime и не считается реализацией целевой V1. Новая работа следует текущему gate из [ROADMAP.md](ROADMAP.md); Phase N+1 не начинается до отдельного gate commit Phase N.

Инструкции в более близком к изменяемому файлу каталоге могут уточнять эти правила. Они не могут ослаблять нормативные инварианты, совместимость, безопасность или фазовые gates.

## Источники и обязательное чтение

Перед любым изменением прочитайте [INVARIANTS.md](INVARIANTS.md), [CONTEXT.md](CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md) и [ROADMAP.md](ROADMAP.md). Затем загрузите только документы ветки задачи:

- persistence, replay, API или migration: [PROTO.md](PROTO.md), [SECURITY.md](SECURITY.md) и связанные [ADR](docs/adr);
- физика, организм, biology или resolution: [WORLD_V1.md](WORLD_V1.md), [coverage matrix](docs/coverage/phase0-coverage-matrix.md) и целевой scenario;
- actions, devices, applications, services, economy или construction: [CIVILIZATION.md](CIVILIZATION.md), [ADR-0012](docs/adr/0012-causal-processes-not-promised-outcomes.md) и [ADR-0013](docs/adr/0013-diegetic-technology-and-institutions.md);
- cognition, consciousness или subjective memory: [MEMORY.md](MEMORY.md) и cognition schemas/fixtures;
- contribution workflow и PR evidence: [CONTRIBUTING.md](CONTRIBUTING.md).

При конфликте используйте authority order из [docs/README.md](docs/README.md): `INVARIANTS.md`, accepted ADR, `ARCHITECTURE.md`/`PROTO.md`, domain specifications/roadmap, затем historical documents. `STAGE_5.md` — только provenance.

## Непереговорные ограничения

- World Engine — единственный authoritative writer physical, biological, neural, digital и institutional state. Вся мутация проходит через `WorldEngine::commit`.
- `Intention`, `PlanHypothesis`, recipe, order, contract, design и application request запускают причинные процессы, но не содержат обещанный outcome. Реализуйте feedback-driven transitions, interruption, partial result, failure и replanning; не outcome functions вроде `cooking()` или `build_house()`.
- Authoritative quantities имеют units либо определённый dimensionless kind. Произвольные health, energy, urgency, cleanliness и другие normalized scores не являются state.
- Каждый mechanism объявляет units, provenance, uncertainty, validity range, conservation, failure policy и validation evidence. Невалидный `MechanismContract` не допускается в runtime.
- Partitioning, restart, downtime, acceleration и worker count не меняют canonical transition stream или state hash.
- Fast replay применяет committed deltas. Audit replay использует exact artifacts по digest и проверяет deltas, conservation и hash. Missing/mismatched artifact, corruption, non-convergence или conservation failure приводит к typed rejection либо `SafeStop`.
- Legacy wire fixtures, packages, DB, snapshots и logs остаются читаемыми и не переписываются. Новая V1 использует отдельную timeline/DB и обратимый compatibility path.
- External side effect требует diegetic permission, host authorization, committed intent и idempotent receipt. Retry сначала выполняет lookup/reconciliation; recovery и replay никогда не вызывают executor повторно.

## Рабочий процесс

1. Зафиксируйте observable outcome, текущую phase, public seam, acceptance evidence, non-goals и rollback до редактирования runtime.
2. Проверьте `git status` и существующие реализации. Сохраняйте пользовательские и unrelated изменения; не форматируйте и не переписывайте их механически.
3. Выберите один узкий vertical slice. Расширяйте существующий deep interface вместо новых параллельных mutation paths, managers и публичных abstractions.
4. Добавьте failing test через public seam и подтвердите ожидаемый red result. Expected values берите из specification, независимого расчёта или измеренного reference, но не из production algorithm.
5. Внесите минимальное изменение до green. Workers и model outputs могут предлагать transitions; authoritative writer повторно валидирует и атомарно commit-ит их.
6. Запустите focused tests, затем все затронутые gates. Просмотрите diff на scope, compatibility, security, units, determinism и documentation drift.
7. Остановитесь после acceptance slice. Следующие biology systems, semantic actions, devices или society modules требуют отдельного scope и phase gate.

Новые production dependencies добавляйте только когда standard library и существующие workspace dependencies недостаточны. Зафиксируйте необходимость, determinism/security impact и rollback. Не вводите schema caps для organisms, cells, neurons или consciousnesses; capacity выражается измеримыми CPU/RAM/storage и честным `CapacityExceeded`.

## TDD и доказательства приёмки

Тестируйте public behavior, durable events, projections и restart/replay, а не private call counts или прямое чтение DB. Для causal kernel и mechanism changes обязательна релевантная часть матрицы:

- один большой и несколько малых canonical interval requests дают одинаковые transitions и state hash;
- reopen/restart продолжает тот же timeline;
- 1 и N workers дают одинаковый результат;
- fast replay совпадает с audit replay;
- retry одного request идемпотентен, conflicting payload отклоняется;
- invalid units, preconditions или conservation отклоняются без partial commit;
- missing/digest-mismatched artifact детерминированно переводит timeline в `SafeStop`;
- legacy archive остаётся byte-identical и читаемым.

Realism claim принимается только с provenance, uncertainty, validity range и validation horizon. Один deterministic seed, replay hash или долгий integration run не доказывает biological realism.

## Совместимость и безопасность изменений

Следуйте `expand -> migrate -> verify -> contract`. Сначала добавьте новый reader/schema/seam рядом со старым, затем докажите совместимость, и только отдельной работой удаляйте временный migration tooling. Не удаляйте legacy reader и не downcast-ите новые events при rollback.

Artifact identity определяется exact bytes digest. Activation candidate artifact требует validation/shadow evidence, explicit approval и committed old/new digests с rollback target. Simulated code не получает прямой host filesystem, network, clock или secrets access.

Не записывайте secrets, private conversations, chain-of-thought, machine-specific paths, runtime DB или generated build output в репозиторий. Security-sensitive ambiguity обрабатывайте по [SECURITY.md](SECURITY.md), а не локальной эвристикой.

## Документация и язык домена

Используйте термины из [CONTEXT.md](CONTEXT.md) точно. Нормативное изменение обновляет single source of truth: invariant/ADR, contract/schema, fixture, coverage matrix и public-seam test по необходимости. Не дублируйте нормативный текст в нескольких документах; связывайте его относительными Markdown links.

Каждый новый публичный Markdown документ должен быть достижим из `README.md` через цепочку ссылок. Historical document получает явный status и не может незаметно стать нормативным.

## Проверка

Минимум для любого изменения:

```bash
git diff --check
cargo fmt --all -- --check
```

Для Rust/runtime/contracts:

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Для documentation-only changes минимум:

```bash
cargo test -p makise-world --test public_repository
```

Если менялись schemas/fixtures, дополнительно запустите `cargo test -p makise-world --test phase0_contracts`. Если менялся C++ client, выполните CMake/CTest commands из [CONTRIBUTING.md](CONTRIBUTING.md).

## Code Review Rules

Сначала проверяйте spec и фазовый scope, затем correctness, authority, units/conservation, deterministic replay, compatibility, privacy/security и failure behavior. Замечание должно указывать конкретный path/line, наблюдаемый риск и минимальный safe fix. Formatting замечания оставляйте автоматическим gates.

Блокируйте change, который добавляет второй authoritative writer, semantic outcome mutation, hidden fidelity downgrade, non-unitful authoritative state, повтор внешнего effect при replay, in-place legacy migration или claims без evidence.

## Условия остановки

Остановитесь и запросите архитектурное решение, если задача требует нового mutation path, меняет authority/consent boundary, не определяет units/conservation, противоречит accepted ADR, выходит за текущую phase либо не имеет compatibility/rollback path. Предпочтительный результат такой остановки — узкий ADR или уточнённый acceptance contract, а не speculative runtime code.
