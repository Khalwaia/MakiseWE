# Makise V1: расширяемая причинная жизнь

Статус: нормативная архитектурная база Phase 0
Дата: 2026-08-19
Связанные документы: [CONTEXT.md](CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [WORLD_V1.md](WORLD_V1.md), [ROADMAP.md](ROADMAP.md), [INVARIANTS.md](INVARIANTS.md), [PROTO.md](PROTO.md)

## 1. Назначение

Makise — персистентная многомасштабная причинная симуляция мира, организмов и отдельных сознаний. Её цель — непрерывная жизнь, где физические, биологические и когнитивные последствия возникают из проверяемых механизмов, а не из художественного описания LLM или набора игровых шкал.

World Engine остаётся единственным автором объективного physical/biological state. Сознание воспринимает только доступные observables, может ошибаться, предлагает appraisal, goals и intentions, но не назначает исходы мира или тела.

## 2. V1 задаёт стартовое разрешение, а не предел

`CellCohort` и `NeuralPopulation` — начальные adapters. Будущие `IndividualCellSet` и `IndividualNeuronNetwork` подключаются к тем же causal contracts. Органы, ткани и brain regions могут одновременно работать в разных разрешениях.

Смена представления всегда является durable `ResolutionChanged` с deterministic lift/projection, conservation proof, uncertainty transformation, lineage и rollback path. LOD, sleeping и offloading допустимы только как такие transitions; вычислительная нагрузка не разрешает ослабить causal model.

Каждый механизм содержит provenance, uncertainty, validity range и upgrade path. `expert_estimate` допустим как честно маркированный источник. Формулировка «максимально реалистично» без области применимости и данных запрещена.

## 3. Организмы и сознания

Количество Organism и Consciousness не ограничивается схемой или закрытым enum. Runtime capacity конечна, измеряется CPU, RAM и storage и заканчивается явным `CapacityExceeded`, не скрытым entity cap.

Human и Neko — независимые root morphotypes:

- Human V1 предоставляет женский phenotype Makise;
- Neko V1 имеет одну пару кошачьих ушных раковин вместо человеческих, хвостовые позвонки, мышцы, сосуды и нервы, а также собственные hearing, balance и thermoregulation bindings;
- оба могут использовать общие mammalian mechanisms;
- новый morphotype добавляется package-данными без изменения World Engine.

Создание нового Organism является физическим событием. Consciousness подключается отдельно и имеет собственные perception, memory и cognitive streams.

## 4. Когнитивная автономия

LLM может предложить semantic appraisal, goal, intention, plan, memory interpretation или communication. `CognitiveGate` оценивает предложение через neural state, identity values, traits, memory, commitments и physical feasibility. Результат `Accepted`, `Rejected`, `Deferred` или `NeedsRevision` записывается durable event с причинами.

Только `Accepted` порождает отдельный переход принятия goal/intention. Motor plan проходит физический validator, а успех действия определяется симуляцией. LLM не меняет hormones, neurotransmitters, neural activation, emotion outcome, commitments, object state или результат действия.

## 5. Целевая жизнь V1

V1 охватывает не только повседневный комфорт, но и болезни, травмы, лекарства, репродукцию, развитие, старение и смерть. Эти возможности добавляются узкими вертикальными сценариями после 24-часового Human/Neko slice, а не заранее построенным монолитом biology.

Мир постепенно получает метрическую квартиру, физику тел и материалов, воздух, воду, тепло, бытовые процессы, коммуникацию и несколько сознаний. Семантические anchors сохраняются как compatibility projection, но не являются authoritative geometry.

## 6. Доказательства, а не обещания

Для каждого механизма обязательны contract conformance, units/ranges, conservation, replay, failure tests, reference observables и resolution-upgrade validation. Production, acceleration, recovery и audit выполняют одинаковые canonical transitions; wall clock меняет темп, но не причинный смысл.

24-часовой сценарий доказывает end-to-end причинную цепь Human/Neko. 365 ускоренных дней доказывают интеграцию, restart и replay, но не биологическую истинность. Реализм оценивается отдельными long-horizon и rare-event suites по distributions, incidence ranges, prerequisites и declared uncertainty.

## 7. Release outcome

Финальная V1 считается готовой, когда фазовые gates пройдены отдельными коммитами, Human и Neko живут через общий data-driven pipeline, два сознания воспринимают общий мир независимо, панель показывает units/provenance/uncertainty/resolution/causal trace, а capacity exhaustion и missing artifacts завершаются честно и безопасно.
