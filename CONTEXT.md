# Makise causal simulation

Единый язык для многомасштабного мира Makise. Термины описывают предметную область, а не текущую реализацию или выбранную технологию.

## Сущности жизни

**Organism**:
Физически непрерывная живая система с собственным телом, границами и причинной историей. Organism может существовать без подключённого Consciousness.
_Avoid_: персонаж, аватар, агент

**Consciousness**:
Отдельный субъективный поток восприятия, памяти, целей и принятых намерений, подключённый к одному Organism в данный момент.
_Avoid_: LLM, brain process, organism

**Morphotype**:
Самостоятельное корневое определение строения, развития и физиологических связей класса организмов.
_Avoid_: раса, Human variant, флаг вида

**Phenotype**:
Конкретизированное в пределах Morphotype сочетание наследуемых и развившихся признаков Organism.
_Avoid_: morphotype, skin

## Причинная модель

**Mechanism**:
Версионированное причинное правило с объявленными входами, выходами, областью применимости, неопределённостью и отказами.
_Avoid_: subsystem score, hidden rule

**CausalGraph**:
Единая сеть authoritative state и Mechanism, в которой последствия могут пересекать масштабы и возвращаться обратной связью.
_Avoid_: linear pipeline, stack of engines

**CausalDomain**:
Область карты CausalGraph, группирующая state и Mechanism по масштабу или причинной роли без задания порядка исполнения или владельца.
_Avoid_: execution stage, isolated engine

**DurableCausalTimeline**:
Упорядоченная сохраняемая история CausalTransition всех CausalDomain; сама не является уровнем симуляции.
_Avoid_: world-events layer, event subsystem

**Resolution**:
Явный способ представления одних и тех же причинных сущностей и величин с заданными правилами refinement, projection и сохранения.
_Avoid_: quality setting, detail level

**ResolutionTransition**:
Явное причинно вызванное изменение Resolution с детерминированным trigger, conservation proof, uncertainty transformation, lineage и rollback.
_Avoid_: automatic LOD, importance boost

**Compartment**:
Ограниченная область, внутри которой состояние вещества или поля считается однородным в пределах заявленной неопределённости.
_Avoid_: arbitrary bucket

**Tissue**:
Организованная совокупность клеток и межклеточной среды с общей структурой и функциями.
_Avoid_: organ, cell list

**CellCohort**:
Агрегированное представление клеток одного совместимого состояния с сохранённым количеством, составом и статистическими моментами.
_Avoid_: cell, normalized health

**NeuralPopulation**:
Агрегированное представление нейронов со стабильными causal ports и объявленной статистикой активности.
_Avoid_: individual neuron, emotion score

**Observable**:
Определённая проекция authoritative state, доступная наблюдателю вместе с единицами, неопределённостью и provenance.
_Avoid_: raw state, UI score

**CausalTransition**:
Атомарно зафиксированное изменение authoritative state, обусловленное перечисленными причинами на каноническом интервале симуляции.
_Avoid_: tick, mutation, patch
