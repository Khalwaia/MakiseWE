---
status: accepted
date: 2026-08-05
updated: 2026-08-19
---

# Separate source, runtime and protected systems

MakiseWE разрабатывается и запускается отдельно от любых других digital-person systems. Source checkout, development state, production runtime, secrets и denied external roots задаются deployment configuration, а не личными paths в коде или документации.

До открытия DB, socket или external connection общий path guard проверяет absolute normalized path, traversal, symlink и mount aliases против denied roots. Production дополнительно использует отдельного OS principal и filesystem permissions. Неоднозначность заканчивается отказом запуска, не fallback.

Следствия: legacy data, diary, memory, messaging sessions и secrets не импортируются неявно; tests используют temporary roots и synthetic identities; repository fixtures не содержат production paths или personal data.
