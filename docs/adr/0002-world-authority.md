---
status: accepted
date: 2026-08-05
updated: 2026-08-19
---

# World Engine is the sole objective-state authority

World Engine остаётся единственным автором objective physical, biological и neural state. `WorldEngine::commit` — единственный mutation path для time, stimuli, model responses, actions, resolution changes и admin intents; transport-specific commands прежнего runtime становятся compatibility inputs этого глубокого module boundary.

Authoritative writer валидирует authority, expected version, canonical interval, contracts, units, conservation, artifact digests и capacity, затем атомарно фиксирует transition и state hash. Retry request идемпотентен; collision одного ID с другим payload отклоняется.

Следствия: Brain, memory, panel, providers и workers только предлагают input или читают projections; timeout не разрешает повторить действие под новым ID; replay остаётся нормативным восстановлением и audit mechanism.
