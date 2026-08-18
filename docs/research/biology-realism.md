# Биологическая реалистичность MakiseWE

Статус: исследовательский аудит текущего репозитория на 19 августа 2026 года. Код не изменялся.

## Краткий вывод

В текущем исполнимом World Engine биологической симуляции нет. Он моделирует положение одного агента по семантическим anchors, занятость ресурсов действий, состояния и количества предметов, погоду и упрощённые проекции среды. Съедание порции уменьшает количество предмета, но не создаёт поступление вещества или энергии в организм. Поэтому текущему runtime нельзя присвоить степень биологической реалистичности: биологическая модель отсутствует.

Целевая архитектура значительно сильнее текущей реализации. Единый причинный граф, величины с единицами, явные границы применимости и неопределённость, баланс вещества и энергии, версионирование артефактов и запрет прямого изменения тела со стороны LLM — правильные основания для научно проверяемой симуляции. Однако Phase 0 содержит только схемы и синтетические fixtures, а не работающие или откалиброванные механизмы. Архитектурная проверяемость уже продумана; биологическая достоверность ещё не показана.

Главный риск плана — Phase 1 требует акустику, теплообмен, нагрузку, дыхание и пищеварение до появления метрической физики, воздуха, тепла, звука и articulated body в Phase 2. Второй риск — попытка получить физиологически правдоподобный 24-часовой контур без renal/fluid, endocrine и musculoskeletal mechanisms, отложенных до Phase 3. Такой Phase 1 может быть полезным интеграционным прототипом, но его результаты следует называть `coarse integration`, а не реалистичной физиологией.

Neko — вымышленный morphotype, для которого не существует эмпирической популяции и цельного набора референсных данных. Данные домашней кошки могут обосновывать отдельные локальные эффекты, но не весь организм Neko. Все переносы должны быть помечены как `species_proxy`, а вымышленные параметры — как `fictional_assumption`, с широкими диапазонами неопределённости.

## Что было проверено

Аудит разделяет три разных уровня доказательств:

1. **Runtime evidence** — состояние и переходы, которые действительно исполняет Rust-код.
2. **Contract evidence** — JSON Schemas и fixtures Phase 0, проверяющие форму будущей модели.
3. **Biological validation** — сравнение выходов численной модели с независимыми экспериментальными данными в объявленном диапазоне условий.

Основные источники внутри репозитория: `README.md`, `ROADMAP.md`, `WORLD_V1.md`, `INVARIANTS.md`, `docs/coverage/phase0-coverage-matrix.md`, `docs/scenarios/phase1-24h-human-neko.md`, исполнимые `world/src/domain.rs`, `world/src/engine.rs` и `world/src/environment.rs`, а также fixtures `contracts/fixtures/morphotypes/human-minimal.json`, `contracts/fixtures/morphotypes/neko-minimal.json`, `contracts/fixtures/mechanisms/minimal-mammalian-transport.json` и `contracts/fixtures/resolutions/cohort-to-individual-cell.json`.

Научные утверждения ниже сверены с оригинальными статьями, официальными спецификациями и документами научных или регуляторных организаций. Обзорные публикации не использовались как основное доказательство.

## Фактическое состояние runtime

`WorldState` не содержит организма, тканей, крови, воды тела, запасов субстратов, температуры тела, сна или neural state. Его authoritative state включает anchors, activities, locks ресурсов, power/open/placement/condition предметов, погоду и время. Названия `Vision`, `Attention`, `Hearing` в `Resource` — взаимоисключающие locks планировщика действий, а не модели сенсорной или нервной физиологии.

Доступные переходы подтверждают этот предел:

- `world.move_to` завершает перемещение после заданного в package `duration_ms`; масса тела, механическая работа, утомление и расход кислорода не вычисляются;
- `object.consume_quantity` только вычитает целое количество из `ObjectCondition.quantity`; масса, вода, питательные вещества, кал и тепло организма не меняются;
- пассивные эффекты линейно изменяют charge, temperature или quantity предметов с фиксированной скоростью;
- `perceived_temperature_millicelsius` — детерминированная эвристика комнаты, окна, холодильника и нагреваемых предметов, а не результат теплового баланса среды или тела;
- свет, звук и запах представлены категориальными или текстовыми cues, а не физическими полями и sensory transduction.

Это честно зафиксировано самим проектом: README называет ядро legacy-compatible и прямо сообщает, что новая физиология не реализована; coverage matrix маркирует biology fixtures как `schema_only` и указывает отсутствие solver validation.

## Что действительно доказывает Phase 0

Phase 0 проверяет архитектурную дисциплину, но не биологию:

- Human и Neko оформлены независимыми root definitions;
- causal ports и state variables имеют единицы;
- механизм переноса кислорода объявляет read/write sets, диапазон применимости, неопределённость и conservation rule;
- переход `CellCohort` к отдельным клеткам сохраняет count, total mass, charge и oxygen amount в синтетическом примере;
- replay, lineage и observable continuity сформулированы как обязательные свойства.

Числа fixtures нельзя использовать как физиологические параметры. Oxygen transfer rate прямо помечен `synthetic_fixture` и исключает claim о biological realism. Human core temperature и body mass имеют `expert_estimate`; Neko auricle area — `expert_estimate` с пометкой `uncalibrated`. В cell fixture средняя масса клетки равна `1e-9 kg`, но пример предназначен только для проверки арифметического сохранения. Само совпадение coarse и fine totals не делает эту величину биологически правдоподобной.

Такое разграничение соответствует MIRIAM: минимально пригодная количественная биомодель должна быть машинно читаемой, однозначно связанной с референсным описанием и достаточно аннотированной для воспроизведения и повторного использования [Le Novère et al., 2005](https://doi.org/10.1038/nbt1156). Текущие contracts создают место для этих сведений, но fixtures пока не содержат библиографического источника, протокола измерения и калибровочного набора для каждого параметра.

## Оценка целевой биологической архитектуры

### Сильные решения

**Причинный граф вместо независимых шкал.** Обратные связи между средой, обменом веществ, нервной системой и действием нужны для homeostasis. Запрет generic `health`/`energy` scores снижает риск скрыть разные причины за одним числом.

**Величины с единицами и явные границы.** Amounts, concentrations, pressures, flows, temperatures и energies позволяют проверять размерность и баланс. CellML 2.0 также делает units, components, interfaces и imports частью формального формата модели; это хороший внешний ориентир для будущих contracts ([официальная спецификация CellML 2.0](https://cellml-specification.readthedocs.io/en/stable/)). SBML формализует compartments, species, reactions, kinetic laws и units для biochemical networks ([SBML Level 3 Version 2 Core](https://doi.org/10.1515/jib-2017-0081)). Заимствование их семантики или импорт проверенных моделей безопаснее, чем создание всей предметной онтологии с нуля.

**Conservation и boundary fluxes.** Требование сохранять массу, количество вещества, заряд и энергию правильно. Для открытого организма это должно быть не «сумма постоянна», а `изменение запаса = входы - выходы + production - consumption` с одним зарегистрированным flux на обеих сторонах связи.

**Mixed resolution с явным transition.** Локальное уточнение ткани разумнее, чем симуляция каждой клетки всего организма. Lineage, projection и error bounds необходимы. Но conservation при смене представления проверяет только отсутствие искусственного создания вещества; он не доказывает правильность распределения клеточных состояний или дальнейшей динамики.

**Разделение cognitive proposal и физического результата.** LLM не должен назначать гормоны, температуру тела или успешность действия. Это сохраняет проверяемую причинность.

**Артефакты, replay и rollback.** Content digests позволяют воспроизвести конкретную версию механизма. Однако одинаковый state hash доказывает детерминизм вычисления, а не соответствие живому организму — проект это уже формулирует правильно.

### Архитектурные пробелы

**Порядок Phase 1/Phase 2.** Phase 1 требует chains `fields → sensory transduction`, `work → oxygen use/CO₂/heat`, акустику, balance task и heat exchange. Phase 2 только затем добавляет metric geometry, mass/inertia, articulated bodies, contacts, atmosphere, heat, acoustics и fluids. Без этих upstream-механизмов Phase 1 неизбежно использует prescribed stimuli и эмпирические surrogates. Это допустимо для vertical slice, если interfaces и validity range прямо говорят, что физическая причина пока задана извне.

**Неполный контур Phase 1.** Вода и пища за 24 часа требуют хотя бы временной модели body-water compartments, renal/excretory boundary, substrate stores и отходов. Exercise требует workload, muscle efficiency, oxygen delivery, CO₂ removal и recovery state. Эти системы в полном виде отложены до Phase 3. Нельзя разрешать отсутствующим выходам просто накапливаться в теле или исчезать.

**Слишком слабая идентифицируемость.** Даже корректная causal topology может иметь больше неизвестных параметров, чем наблюдаемых величин. Contracts должны хранить не только uncertainty, но и способ идентификации параметра, корреляции между параметрами, calibration dataset и отдельный validation dataset.

**Синтетический lift создаёт возможные, а не известные клетки.** Deterministic seeded sampling хорошо для replay, но конкретные индивидуальные клетки после lift не наблюдались до refinement. Их provenance должен отличать measured lineage от generated realization. Fine simulation не может задним числом заявлять индивидуальную историю, которой не было в coarse state.

**Абсолютные tolerances без численного анализа.** Tolerances уровня `1e-15 kg` или точный cell count могут быть полезны для маленького fixture, но должны выводиться из solver precision, масштаба состояния и conditioning модели. Фиксированный абсолютный порог не переносится автоматически с четырёх клеток на орган.

## Реалистичность запланированных механизмов

### Дыхание и перенос кислорода

Amount-flow ports между circulation и tissue — хорошая граница модуля, но текущего набора `circulation.oxygen-amount`, `tissue.oxygen-amount` и `cell-count` недостаточно даже для минимальной физиологии нагрузки. Нужны как минимум объёмы compartments или concentrations, blood flow, arterial/venous oxygen content и связь с partial pressure. Насыщение гемоглобина нелинейно зависит от PO₂; положение кривой зависит от температуры и pH, что показано в исходных уравнениях и измерительной процедуре [Severinghaus, 1979](https://doi.org/10.1152/jappl.1979.46.3.599). Следовательно, thermoregulation, CO₂/acid-base и oxygen delivery нельзя соединять только независимыми линейными rates.

Для Phase 1 допустима lumped модель, если она валидируется только для покоя и объявленного диапазона умеренной нагрузки. Термины «одинаковая умеренная нагрузка» для Human и Neko нужно заменить измеримой постановкой: одинаковая внешняя mechanical power, одинаковая power на kg или одинаковая доля индивидуальной aerobic capacity — это разные эксперименты.

### Метаболизм, работа и тепло

Цепь `O₂ use + CO₂ production → metabolic power → mechanical work + heat` научно обоснована. Оригинальная работа Weir выводит расчёт metabolic rate из gas exchange и nitrogen correction [Weir, 1949](https://doi.org/10.1113/jphysiol.1949.sp004363). Значит Phase 1 должен согласованно хранить VO₂/VCO₂, chemical energy источника, полезную работу и тепло, а не независимо подгонять четыре выхода.

Пища не может сразу становиться общим `energy`. Минимально нужны macronutrient amounts, доступный substrate pool, absorption fluxes и неусвоенный остаток. На горизонте 24 часов допустима агрегированная digestion model, но только с балансом массы и калибровкой time-series после еды.

### Терморегуляция

Одна reference core temperature не является thermoregulation model. Классическая динамическая модель NASA использовала 25 thermal nodes: compartments сегментов тела, central blood, metabolic heat, conductive и blood-mediated transfer, а на поверхности radiation, convection и evaporation; controller менял heat production, blood flow и sweating. Модель сравнивалась с реальными температурными step exposures и exercise при 25%, 50% и 75% maximum aerobic capacity ([Stolwijk, NASA CR-1855, 1971](https://ntrs.nasa.gov/citations/19710023925)).

Phase 1 не обязан повторять 25-node модель. Минимально защитимый вариант — отдельные core и skin temperatures, body heat capacity, blood coupling, metabolic heat, convection/radiation/evaporation, clothing/fur и effector limits. До Phase 2 коэффициенты environment exchange останутся prescribed surrogates.

Число `310.15 K` годится как стартовая точка, но не set point, к которому тело всегда возвращается. В эксперименте на 148 здоровых взрослых oral temperature имела суточный минимум около 06:00, максимум 16:00–18:00 и среднюю амплитуду около 0.5 °C [Mackowiak et al., 1992](https://pubmed.ncbi.nlm.nih.gov/1302471/). Site of measurement, circadian phase, activity и phenotype должны входить в semantics наблюдаемой температуры.

### Сон и circadian dynamics

Пара `asleep/awake` и произвольный recovery score недостаточны. Оригинальная two-process model разделяет homeostatic process S, растущий во время бодрствования и падающий во сне, и circadian thresholds, управляемые pacemaker; параметры были получены из EEG power и sleep-deprivation experiments [Daan, Beersma & Borbély, 1984](https://doi.org/10.1152/ajpregu.1984.246.2.R161). Минимальная Phase 1 модель должна отдельно хранить sleep pressure и circadian phase/amplitude, а light history использовать как вход zeitgeber. «Accepted sleep intention» может создавать условия сна, но не должна гарантировать немедленный physiological sleep transition.

### Neko: слух, хвост и теплообмен

Данные домашней кошки подтверждают отдельные эффекты. Behavioral audiograms двух кошек дали диапазон 48 Hz–85 kHz при 70 dB SPL [Heffner & Heffner, 1985](https://doi.org/10.1016/0378-5955(85)90100-5). Это не готовая transfer function для Neko: диапазон не определяет HRTF, localization, loudness, masking или neural transduction, а геометрия головы и pinnae вымышленного morphotype неизвестна.

Хвост действительно может участвовать в динамическом равновесии: в опыте четыре кошки компенсировали внезапное боковое смещение узкой балки движением хвоста; после sacrocaudal transection падения участились [Walker, Vierck & Ritz, 1998](https://doi.org/10.1016/S0166-4328(97)00101-0). Этот результат обосновывает coupling в конкретной задаче, но не утверждение, что хвост универсально определяет баланс. Для прямоходящего Neko понадобятся собственные body dynamics, center of mass и control policy; quadrupedal cat coefficients напрямую не переносятся.

Привязка thermoregulation к одной `auricle-convective-area` слишком слаба. У кошек heat loss связан с hydration и hypothalamic response: изменение plasma osmolality экспериментально меняло evaporative heat loss и body temperature при 38 °C [Baker & Doris, 1982](https://doi.org/10.1113/jphysiol.1982.sp014282). Нужны как минимум perfusion, surface temperature, fur/skin insulation, air velocity и respiratory evaporation. Значение площади `0.018 m²` остаётся вымышленным до определения геометрии и измеримого target phenotype.

## Как доказать реализм, не только корректность кода

FDA рекомендует начинать оценку mechanistic simulation с question of interest, context of use и model risk, затем отдельно собирать code verification, calculation verification, validation и uncertainty evidence ([FDA final guidance, 2023](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/assessing-credibility-computational-modeling-and-simulation-medical-device-submissions)). Для MakiseWE это означает следующий gate перед любым claim о биологическом реализме.

1. Для каждого mechanism определить измеримый context of use: species/phenotype, возраст, поза, питание, ambient range, activity range и simulation horizon.
2. Для каждого параметра хранить DOI/URL, таблицу или figure, population, measurement method, unit conversion, fitted value, confidence interval и license. Категории `expert_estimate` без источника недостаточно.
3. Разделить calibration и validation datasets. Подгонка и проверка на одном 24-hour trace не являются независимой валидацией.
4. Проверять временные ряды и derived observables, а не только конечный state: VO₂, VCO₂, core/skin temperature, body-water balance, absorbed substrate, sleep timing и response latency.
5. Для каждого flux записывать source, sink и boundary; тестировать residual массы, атомов ключевых веществ, энергии, воды и заряда на каждом canonical interval.
6. Выполнять sensitivity analysis и propagation of parameter uncertainty. Wide Neko uncertainty должна давать wide predictions, а не точные числа из deterministic seed.
7. Проверять resolution replacement на динамике: одинаковы должны быть не только totals в момент lift, но и заявленные observables после одинакового stimulus в coarse и fine моделях в течение validity horizon.
8. Маркировать происхождение отдельно: `measured_human`, `measured_domestic_cat`, `species_proxy`, `expert_estimate`, `synthetic_fixture`, `fictional_assumption`. Neko claim никогда не должен автоматически наследовать статус cat evidence.
9. Добавить negative biological tests: extreme input должен приводить к `outside_validity_range`/`SafeStop`, а не к численно стабильному, но физиологически бессмысленному продолжению.

Минимальный набор независимых Phase 1 experiments:

- Human: rest/fasting baseline; стандартизированная еда; заданная mechanical workload и recovery; ambient temperature step; обычный сон и sleep deprivation;
- domestic-cat proxy: behavioral hearing thresholds, perturbed-beam tail task, heat/hydration response;
- cross-mechanism: heat and gas balance during exercise, water balance during heat exposure, circadian modulation of temperature and sleep;
- numerical: interval partitioning, restart/replay, conservation residuals и coarse/fine observable divergence.

## Итоговая оценка

| Объект оценки | Состояние | Вывод |
|---|---|---|
| Текущий Rust runtime | Исполняется и тестируется | Биологии нет; реализм организма не оценивается |
| Phase 0 contracts | Schemas и synthetic fixtures | Сильная база для проверяемости, но не validation evidence |
| Phase 1 scenario | План | Хороший integration slice; физиологически неполон и зависит от ещё не реализованной физики |
| Human target | Концепция | Реализуем как coarse organ-level model при строгой калибровке |
| Neko target | Вымышленный morphotype | Цельная эмпирическая валидация невозможна; возможна только валидация компонентов и явных proxies |
| Adaptive cell/neural resolution | Contract examples | Архитектурно разумно; conservation и replay не доказывают правильную microstate dynamics |

Рекомендуемая формулировка проекта до появления данных: **«детерминированная, причинно и размерностно проверяемая многомасштабная архитектура для будущей физиологической симуляции»**. Формулировка **«биологически реалистичная симуляция Human и Neko»** станет обоснованной только после механизм-специфической калибровки, независимой validation и опубликованных validity envelopes.

## Первичные источники

1. Le Novère N. et al. Minimum information requested in the annotation of biochemical models (MIRIAM). *Nature Biotechnology*. 2005. [DOI 10.1038/nbt1156](https://doi.org/10.1038/nbt1156).
2. CellML Editorial Board. *CellML 2.0 Specification*. [Официальная спецификация](https://cellml-specification.readthedocs.io/en/stable/).
3. Hucka M. et al. The Systems Biology Markup Language (SBML): Language Specification for Level 3 Version 2 Core Release 2. *Journal of Integrative Bioinformatics*. 2019. [DOI 10.1515/jib-2017-0081](https://doi.org/10.1515/jib-2017-0081).
4. U.S. Food and Drug Administration. *Assessing the Credibility of Computational Modeling and Simulation in Medical Device Submissions*. Final Guidance, November 2023. [FDA](https://www.fda.gov/regulatory-information/search-fda-guidance-documents/assessing-credibility-computational-modeling-and-simulation-medical-device-submissions).
5. Stolwijk J.A.J. *A Mathematical Model of Physiological Temperature Regulation in Man*. NASA CR-1855, 1971. [NASA Technical Reports Server](https://ntrs.nasa.gov/citations/19710023925).
6. Weir J.B. de V. New methods for calculating metabolic rate with special reference to protein metabolism. *The Journal of Physiology*. 1949;109:1–9. [DOI 10.1113/jphysiol.1949.sp004363](https://doi.org/10.1113/jphysiol.1949.sp004363).
7. Severinghaus J.W. Simple, accurate equations for human blood O₂ dissociation computations. *Journal of Applied Physiology*. 1979;46:599–602. [DOI 10.1152/jappl.1979.46.3.599](https://doi.org/10.1152/jappl.1979.46.3.599).
8. Mackowiak P.A., Wasserman S.S., Levine M.M. A critical appraisal of 98.6 degrees F. *JAMA*. 1992;268:1578–1580. [PubMed 1302471](https://pubmed.ncbi.nlm.nih.gov/1302471/).
9. Daan S., Beersma D.G.M., Borbély A.A. Timing of human sleep: recovery process gated by a circadian pacemaker. *American Journal of Physiology*. 1984;246:R161–R183. [DOI 10.1152/ajpregu.1984.246.2.R161](https://doi.org/10.1152/ajpregu.1984.246.2.R161).
10. Heffner R.S., Heffner H.E. Hearing range of the domestic cat. *Hearing Research*. 1985;19:85–88. [DOI 10.1016/0378-5955(85)90100-5](https://doi.org/10.1016/0378-5955(85)90100-5).
11. Walker C., Vierck C.J. Jr., Ritz L.A. Balance in the cat: role of the tail and effects of sacrocaudal transection. *Behavioural Brain Research*. 1998;91:41–47. [DOI 10.1016/S0166-4328(97)00101-0](https://doi.org/10.1016/S0166-4328(97)00101-0).
12. Baker M.A., Doris P.A. Control of evaporative heat loss during changes in plasma osmolality in the cat. *The Journal of Physiology*. 1982;328:535–545. [DOI 10.1113/jphysiol.1982.sp014282](https://doi.org/10.1113/jphysiol.1982.sp014282).
