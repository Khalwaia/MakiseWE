---
status: accepted
date: 2026-09-05
---

# Canonical physiology replay и совместимость timeline

Для следующего Phase 3.2 среза принят новый явно выбираемый формат timeline: canonical transitions на каждой секундной границе и отдельная версия запроса. Это реализация требований [ADR-0009](0009-canonical-simulation-time.md) и [ADR-0010](0010-content-addressed-artifacts.md), а не изменение authority или физиологии. Решение не разрешает activation artifacts и не объявляет runtime готовым.

## Проблема и выбор

Существующий [WorldEngine](../../causal-kernel/src/lib.rs) исполняет физиологию по секундам, но сохраняет одну aggregate transition на `commit`. `sequence`, `timeline_version` и lookup receipt связаны отношением 1:1. Public test [committed_intervals_are_durable_and_readable_after_reopen](../../causal-kernel/tests/audit_replay.rs) закрепляет две записи для запросов 3 s и 2 s. Замена их пятью записями в том же формате меняет pagination, optimistic concurrency и receipt semantics.

Reader и поведение существующего формата сохраняются; новый формат выбирается явно при создании отдельной timeline через `OpenSpec`. При reopen формат определяется durable metadata, несовместимое требование отклоняется. Автоматическое переключение существующей DB запрещено. Альтернатива — изменить смысл старых `sequence` и receipts — отвергнута из-за несовместимости; генерация посекундных events при чтении отвергнута, поскольку они не были committed.

## Запросы и transitions

`WorldEngine::commit` остаётся единственным authoritative writer. В новом формате один успешный запрос увеличивает request version на один, но может атомарно добавить несколько canonical transitions. Receipt сохраняет исходную request version, диапазон event sequence и конечный hash. Retry возвращает тот же диапазон; conflicting payload отклоняется до изменений. Event cursor использует event sequence и не является request version.

Продвижение на N секунд фиксирует интервалы `[t,t+1)`, ..., `[t+N-1,t+N)`; intake — отдельную transition на текущей границе. Существующий `advance_to` фактически принимает длительность; эта работа сохраняет её смысл. Запрос большого интервала полностью проверяется до публикации: ошибка на последней секунде не оставляет committed prefix, clock, snapshot, receipt или новую регистрацию artifacts. Лимиты временного хранения рассчитываются через capacity admission, а не schema cap.

Request ID и receipt association остаются durable transport metadata отдельно от canonical causal payload. Сравнение разных partitions включает порядок, интервалы, причинные inputs, deltas, artifact digests, uncertainty, conservation и state hashes; исключает только request correlation/version. Stimulus identity и порядок intake на одной границе остаются причинными данными и не исключаются из сравнения. В новом формате `events` выдаёт только committed canonical records, без пересчёта и синтетического дробления старых events.

## Архивируемое исполнение

Manifest новой timeline должен связывать по exact bytes digest следующие зависимости фактически исполняемого advance:

| Компонент | Что должно быть закреплено в artifact |
|---|---|
| Начальное состояние и morphotype | Quantities с units, physiological parameters, schema и resolution; нельзя восстанавливать morphotype по совпадению blood values |
| Scheduler, circadian и sleep debt | Порядок переходов, окна времени, metabolic demand, правила debt и integer rounding |
| Digestion | Перенос из buffer в chemical store, скорость и capacity constraints |
| Ambient exchange | Thermal solver ABI, capacities, conductance, validity и rounding |
| Cardiorespiratory exchange | O2/CO2 amounts, respiratory quotient, diffusion, boundary fluxes и error accounting |
| Renal intake/excretion и blood coupling | Water/Na transfers, resting bounds, corrective rates, volume/MAP response |
| Metabolism | Chemical-to-thermal transfer, overdraft и conservation |

Каждый dependency требует полного `MechanismContract`, executable program/parameter artifact и validation evidence. JSON fixture с mechanism ID или digest исходного Rust-файла не является executable program. Принята versioned deterministic ABI: executor читает архивные параметры, dispatch проверяет ABI и совместимость exact artifact, прежняя ABI-семантика сохраняется для старых историй. Unknown fields/ABI, missing dependency и digest mismatch отклоняются. Реализация ABI и её contract tests — первый runtime этап после принятия решения; production activation остаётся отдельной процедурой [PROTO.md](../../PROTO.md#3-required-event-families).

Fast replay применяет committed deltas, проверяя units, `before`, уникальность quantities, clock/state transitions и hash-chain, без execution. Audit запускает всю архивную dependency graph из предыдущего проверенного состояния, затем сравнивает deltas, uncertainty, conservation и hash. Ошибка приводит к typed rejection либо durable `SafeStop`; запись diagnostic не меняет организм и прежние events. Поддержка лишь известных bytes текущего renal fixture является временным compatibility guard, а не завершённой реализацией этой ABI.

Conservation evidence нового формата содержит отдельные результаты для воды body+urine (`mm3`), натрия plasma+urine (`umol`), энергии chemical+digestive+core+ambient (`uJ`) и объявленных O2/CO2 boundary/reaction fluxes (`umol`). Для exact integer transfers residual равен нулю; rounding и стехиометрические допуски gas exchange должны быть явно выведены и проверены в contract до его admission. Одного water report или `NotEvaluated` для всего advance недостаточно. Новые conservation поля требуют versioned reader; старые single-report records остаются читаемыми.

Дополнительный compatibility defect исходного encoder: renal sodium deltas помечены как `centi_umol`, хотя `RenalState` хранит значения в `umol` без пересчёта масштаба. Новые records обязаны использовать согласованный `umol` contract. Старые bytes нельзя исправлять или молча переобозначать при чтении; reader сохраняет исходную запись, а dimensional validation должна явно сообщать несовместимость. Успешный replay hash сам по себе не доказывает корректность units. Исправление writer/reader требует отдельного red/green этапа и compatibility fixture до заявления о полной поддержке нового формата; evidence этого этапа записано в [плане Phase 3](../plans/0006-phase3-everyday-physiology.md#исправление-units-renal-sodium).

## Acceptance и граница среза

Public seams: `open`, `commit`, `events`, `project`, `fast_replay`, `audit_replay` и чтение `safe_stop`. Начальный сценарий: baseline, intake 250 000 mm³ / 35 000 µmol, продвижение 60 s, повторный такой intake. Независимые anchors coarse contract: total-body water 42 498 980 mm³, urine water 1 020 mm³, plasma Na 489 940 µmol, urine Na 60 µmol; суммы соответственно 42 500 000 mm³ и 490 000 µmol. Это synthetic acceptance, не empirical validation.

Приёмка требует следующих доказательств:

- 1×60 s и 60×1 s дают одинаковые canonical records и конечный hash; второй intake использует состояние после excretion.
- Reopen между запросами и оба replay режима совпадают; retry после reopen возвращает прежний receipt/range, conflict ничего не меняет.
- Missing/changed artifact, same-ID artifact с другим корректным digest, неверные units/`before`, conservation и hash детерминированно отклоняются. Mutation storage в тестах допустима только для fault injection; результат проверяется public API.
- Сбой последнего шага большого запроса не оставляет committed prefix; сохранённый старый архив остаётся byte-identical и читаемым.
- 1/N workers предлагают работу одной timeline, writer канонически валидирует и редуцирует её. Запуск N независимых engines и сравнение только часов не считается worker parity. Пока такой режим не реализован, этот пункт остаётся открытым, без фиктивной настройки worker count.

Новые органы, hormones, изменение physiological coefficients и performance optimization не входят в replay срез. Пользовательское решение от 2026-09-07 включает executable renal resolution upgrade в общий scope 3.2; его acceptance boundary записана в [плане Phase 3](../plans/0006-phase3-everyday-physiology.md#расширение-scope-32-от-2026-09-07). Оно не меняет этот replay contract и не разрешает обход artifact admission/activation. Новые schemas, fixtures, readers и runtime проходят отдельные red/green этапы внутри Phase 3.2; gate 3.2 остаётся открытым до полного evidence.

## Migration и rollback

Expand добавляет новый format reader и explicit creation option рядом со старым. Verify использует заранее сохранённые fixtures прежней timeline: прежние events, receipts, projections и архивные bytes не переписываются. Старые aggregate advance histories остаются читаемыми; audit без исторических executable dependencies честно завершается typed rejection/`SafeStop`, а не пересчитывает их текущими механизмами.

Первый новый сценарий начинается от новой genesis. Перенос живого состояния в новую timeline не входит в решение и потребует отдельного snapshot/provenance contract. Rollback возвращает предыдущий executable и его прежнюю timeline; новую DB сохраняет отдельно. Не допускаются downcast новых events, запись старым executable в новый формат или заявление о непрерывности истории между независимыми genesis.
