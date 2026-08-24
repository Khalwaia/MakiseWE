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

**FidelityEnvelope**:
Объявленная область причинной и эмпирической достоверности Mechanism по состояниям, масштабам, ошибке и validation evidence.
_Avoid_: максимальная реалистичность без границ, quality preset

## Действия и процессы

**Intention**:
Принятая Consciousness цель, разрешающая попытку действия, но не содержащая физический результат и не гарантирующая успех.
_Avoid_: command outcome, completed action

**PlanHypothesis**:
Изменяемая гипотеза о доступной последовательности affordances для продвижения к Intention с учётом текущих наблюдений.
_Avoid_: гарантированный сценарий, recipe execution

**ControlEpisode**:
Сохраняемая closed-loop попытка реализовать Intention через perception, control, Mechanism и replanning; хранит текущее состояние, а не обещанный финальный mutation.
_Avoid_: activity timer, delayed mutation

**PhysicalProcess**:
Развивающаяся во времени причинная динамика материи, энергии, полей или тела, способная дать частичный, побочный либо неуспешный результат.
_Avoid_: semantic action function, scripted outcome

## Техника и цифровой мир

**DigitalDevice**:
Физический объект с вычислительным состоянием, чьи execution, sensors, radios, energy, heat, wear и failures принадлежат одному CausalGraph.
_Avoid_: UI inventory item, external computer

**CodeArtifact**:
Неизменяемые bytes исходного, собранного или исполняемого кода с digest, provenance, dependencies и lineage.
_Avoid_: mutable app version, LLM-described program

**CapabilityGrant**:
Отзываемое ограниченное право конкретного субъекта или CodeArtifact запрашивать определённое наблюдение либо воздействие над заданным ресурсом.
_Avoid_: global permission, direct world access

**ExternalEffectIntent**:
Авторизованный запрос на потенциально необратимое воздействие вне симуляции, ещё не являющийся доказательством выполнения.
_Avoid_: external side effect, success event

**ExternalEffectReceipt**:
Идемпотентное подтверждение фактического внешнего результата, используемое replay без повторного выполнения воздействия.
_Avoid_: assumed success, repeated replay call

## Институты и производство

**Organization**:
Институциональная структура ролей, полномочий, имущества, правил и обязательств; не имеет собственного Consciousness без действующих участников.
_Avoid_: collective mind, autonomous person

**ServiceOffering**:
Публичное предложение выполнить определённую работу при объявленных условиях, не гарантирующее появление результата.
_Avoid_: service function, automatic fulfillment

**ServiceContract**:
Принятое сторонами множество взаимных прав, обязанностей, критериев приёмки и последствий нарушения.
_Avoid_: completed order, guaranteed outcome

**Possession**:
Фактический физический контроль над объектом, не обязательно совпадающий с институциональным правом на него.
_Avoid_: ownership, TitleClaim

**TitleClaim**:
Институционально признанное притязание субъекта на объект или территорию, существующее отдельно от Possession.
_Avoid_: physical control, immutable ownership

**DesignArtifact**:
Версионированное описание намеренной конструкции, не тождественное фактически построенной геометрии и состоянию объекта.
_Avoid_: finished building, authoritative geometry
