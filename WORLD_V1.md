# Мир и воплощение Makise V1

Статус: нормативная целевая спецификация; реализация по фазовым gates
Дата: 2026-08-19
Связанные документы: [VISION.md](VISION.md), [ARCHITECTURE.md](ARCHITECTURE.md), [ROADMAP.md](ROADMAP.md), [INVARIANTS.md](INVARIANTS.md)

## 1. Объективный мир

World Engine является единственным автором геометрии, вещества, полей, объектов, organisms и biological/neural state. Любой внешний stimulus, action, administrative intent или model response становится причиной только после `commit`. Observers получают projections, ограниченные доступом, сенсорной достижимостью и uncertainty.

V1 начинается в квартире, но apartment boundary не является границей causal model. Семантические rooms/anchors текущего runtime сохраняются как compatibility projection. Phase 2 вводит метрическую 3D geometry, materials, mass, inertia, articulated bodies и active physics islands; authoritative действия перестают зависеть от `duration_ms`, cleanliness/charge/resource scores.

## 2. Causal domains и события

Мир, organisms и consciousnesses образуют единый causal graph, а не линейный L0–L7 pipeline. Physical world, organism, tissue/cellular, molecular/biochemical, neural/brain, consciousness, motor control и physical action — пересекающиеся causal domains. Mechanisms могут соединять несколько domains через stable causal ports; scheduler не обязан посещать каждый domain на каждом interval.

Durable causal timeline не является слоем `WORLD EVENTS`. Дождь создают environment/physical mechanisms; открытие двери — cognitive, motor, contact и articulated-object mechanisms; падение — gravity/contact mechanisms. Их committed transitions попадают в общую timeline вместе с causes, artifact digests, unit deltas, uncertainty, conservation и state hash.

## 3. Среда

Целевая квартира моделирует только механизмы, нужные проверяемым сценариям:

- airflow, temperature, humidity, gases и aerosols;
- acoustics, lighting и odors;
- вода, электричество и physical food transformations;
- contacts, grasp, carry, fall, spill, heat и cleaning;
- clothing materials, insulation, moisture, contamination, wear и fit.

Каждая величина имеет units, provenance, uncertainty и validity range. Упрощённая модель допустима, если её causal ports стабильны и определён upgrade path. Новая комната или предмет не требует нового engine branch.

## 4. Organism composition

Organism собирается из `MorphotypeDefinition`, phenotype parameters и набора mechanisms. Anatomy graph связывает organs, tissues, compartments, vasculature, innervation и development. Разные узлы графа могут иметь разные active resolutions.

Начальные adapters:

- `CellCohort` агрегирует совместимые клетки и их conserved quantities;
- `NeuralPopulation` агрегирует нейроны и статистику сигналов;
- будущие individual representations реализуют те же causal inputs/outputs/observables.

Состояние organism не является одной health/energy шкалой. Используются unitful mass, amounts, concentrations, pressures, flows, temperatures, energies, counts и определённые безразмерные quantities.

Resolution повышается или понижается только через explicit causally triggered `ResolutionChanged`. Trigger задаётся contract validity/uncertainty, divergent lineage, требованиями causal interaction или validation policy, а не расстоянием до камеры или произвольной важностью. Lift/projection сохраняют declared quantities, moments, lineage и observables внутри error bounds.

## 5. Human и Neko

Human и Neko — самостоятельные root packages, а не варианты runtime type.

Human V1 включает женский phenotype Makise и собственные anatomy, development, organ bindings, physiological parameters и fixtures.

Neko V1 самостоятельно задаёт mammalian anatomy и отличается как минимум:

- одной парой кошачьих external auricles вместо человеческих;
- tail vertebrae, muscles, vessels и nerves;
- morphology-specific hearing transfer, vestibular/balance coupling и thermoregulation;
- собственными development и validation fixtures.

Общие mammalian mechanisms переиспользуются по contract/digest. Compatibility reproduction задаётся morphotype data, не условием в engine.

## 6. Жизненный охват V1

Фазы последовательно вводят:

1. минимальные digestion, circulation, respiration, metabolism, thermoregulation, sleep и sensory transduction;
2. physical embodiment и квартиру;
3. everyday physiology: cardiovascular, respiratory, renal, digestive, endocrine, skin, musculoskeletal, excretion и microbiome;
4. cell turnover, immunity, infection, wounds, cancer, drugs, organ failure и death;
5. genetics, gametes, fertility, pregnancy, fetal development, birth, growth и aging;
6. replaceable neural resolution, neurotransmission, autonomic/endocrine coupling, affect episodes, learning и memory consolidation;
7. полную повседневную жизнь и несколько consciousnesses.

Механизм добавляется только когда нужен вертикальному сценарию и приходит вместе с contract, observables, validation data и upgrade path.

## 7. Время и движение причин

Simulation time представляется каноническими интервалами. Event boundaries, mechanism scheduling и deterministic random streams одинаковы для 1:1, acceleration, restart, recovery и audit replay. Wall clock управляет скоростью production, но не меняет переходы.

Действие начинается с принятого intention и validated motor plan. Оно развивается closed-loop через sensors, contacts и body dynamics; World Engine определяет успех и побочные последствия. Downtime не создаёт решений или восприятий от имени Consciousness.

## 8. Несколько сознаний

Каждое Consciousness имеет отдельные perception, cognitive decision, memory и privacy streams. Несколько сознаний видят один objective world с разных тел и точек доступа. Shared world event не становится одинаковым субъективным опытом автоматически.

Commitments принимаются только CognitiveGate соответствующего сознания. Интимные и репродуктивные действия требуют принятых intentions всех участников и физической допустимости; administrative authority не заменяет consent.

## 9. Capacity и fidelity

Schema не ограничивает число organisms, cells, neurons или consciousnesses. Admission control сравнивает declared compute estimate с доступными ресурсами. Workstation release target: Human + Neko, два активных Consciousness, production 1:1 на 16 CPU cores/32 GB RAM, World Engine не более 24 GB.

При исчерпании capacity система сообщает `CapacityExceeded`. Она не пропускает mechanisms, не меняет resolution и не переводит active entities в sleeping/offloaded representation без явного transition и conservation proof.

## 10. Validation horizons

24-часовой Human/Neko slice — первый end-to-end contract. 365 accelerated days проверяют seasons, sleep, metabolism, reproductive cycles, infections, multiple organisms и replay integration, но не служат доказательством realism.

Biological validation использует 10/30/80-year aging runs, cell turnover, mutation/tumor, chronic disease, fertility/pregnancy/development, neuroplasticity/habits/memory и morphotype lifespan differences. Rare-event deterministic ensembles охватывают arrhythmia, thrombosis, anaphylaxis, sepsis, malignancy, toxicity, pregnancy complications, trauma, Neko ear/tail injury, organ failure и death.
