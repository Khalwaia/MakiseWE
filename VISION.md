# Makise V1: расширяемая причинная жизнь

Статус: нормативная архитектурная база Phase 0
Дата: 2026-08-19
Связанные документы: [CONTEXT.md](CONTEXT.md), [ARCHITECTURE.md](ARCHITECTURE.md), [WORLD_V1.md](WORLD_V1.md), [CIVILIZATION.md](CIVILIZATION.md), [ROADMAP.md](ROADMAP.md), [INVARIANTS.md](INVARIANTS.md), [PROTO.md](PROTO.md)

## 1. Назначение

Makise — персистентная многомасштабная причинная симуляция мира, организмов, отдельных сознаний и создаваемой ими цивилизации. Её цель — непрерывная жизнь, где физические, биологические, когнитивные, цифровые и institutional последствия возникают из проверяемых механизмов, а не из художественного описания LLM или набора игровых шкал.

Simulation образует единый causal graph с обратными связями и mixed resolution. Physical world, organism, tissue/cellular, molecular/biochemical, neural/brain, consciousness, motor control, physical action, digital/computation и institutional/economic state являются causal domains, а не последовательными стадиями tick или независимыми engines. Durable causal timeline хранит переходы всех domains и сама не является `WORLD EVENTS` layer.

World Engine остаётся единственным автором authoritative physical, biological, neural, digital и institutional state. Сознание воспринимает только доступные observables, может ошибаться, предлагает appraisal, goals и intentions, но не назначает исходы мира, тела, программы, услуги или строительства.

## 2. V1 задаёт стартовое разрешение, а не предел

`CellCohort` и `NeuralPopulation` — начальные adapters. Будущие `IndividualCellSet` и `IndividualNeuronNetwork` подключаются к тем же causal contracts. Органы, ткани и brain regions могут одновременно работать в разных разрешениях.

Смена представления всегда является explicit causally triggered durable `ResolutionChanged` с deterministic trigger, lift/projection, conservation proof, uncertainty transformation, lineage и rollback path. Trigger следует из contract validity, uncertainty, divergent lineage или требований взаимодействия, а не из субъективной «важности». LOD, sleeping и offloading допустимы только как такие transitions; вычислительная нагрузка не разрешает ослабить causal model.

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

Только `Accepted` порождает отдельный переход принятия goal/intention. Intention запускает closed-loop `ControlEpisode`, не обещанный outcome; каждый физический, цифровой или institutional шаг проверяется текущим state и feedback. LLM не меняет hormones, neurotransmitters, neural activation, emotion outcome, commitments, object state, code execution или результат действия.

## 5. Целевая жизнь V1

V1 охватывает не только повседневный комфорт, но и болезни, травмы, лекарства, репродукцию, развитие, старение и смерть. Эти возможности добавляются узкими вертикальными сценариями после 24-часового Human/Neko slice, а не заранее построенным монолитом biology.

Мир постепенно получает метрическую geometry, физику тел и материалов, воздух, воду, тепло, бытовые процессы, технику, исполняемые приложения, networks, organizations, services, производство, строительство и несколько сознаний. Персонажи могут создавать software, marketplace releases, компании, дома и датацентры через фактический код, contracts, material flows и work. Семантические action names, anchors, orders и designs остаются projections или intentions и не создают outcome.

## 6. Доказательства, а не обещания

Для каждого механизма обязательны contract conformance, units/ranges, conservation, replay, failure tests, reference observables и resolution-upgrade validation. Production, acceleration, recovery и audit выполняют одинаковые canonical transitions; wall clock меняет темп, но не причинный смысл.

24-часовой сценарий доказывает end-to-end причинную цепь Human/Neko. 365 ускоренных дней доказывают интеграцию, restart и replay, но не биологическую истинность. Реализм оценивается отдельными long-horizon и rare-event suites по distributions, incidence ranges, prerequisites и declared uncertainty.

## 7. Release outcome

Финальная V1 считается готовой, когда фазовые gates пройдены отдельными коммитами, Human и Neko живут через общий data-driven pipeline, два сознания воспринимают общий мир независимо, панель показывает units/provenance/uncertainty/resolution/causal trace, а capacity exhaustion и missing artifacts завершаются честно и безопасно.
