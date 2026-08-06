# Инварианты Makise

Статус: обязательный контракт безопасности V1  
Дата: 2026-08-05  
Связанные документы: [VISION.md](VISION.md), [ARCHITECTURE.md](ARCHITECTURE.md), [SECURITY.md](SECURITY.md), [PROTO.md](PROTO.md), [ROADMAP.md](ROADMAP.md)

`MUST` означает условие, нарушение которого блокирует запуск или релиз. `MUST NOT` означает безусловный запрет.

## 1. Изоляция Мины

1. Makise MUST NOT читать, создавать, изменять, перемещать или удалять что-либо внутри `/home/artem/kuni_run`.
2. В частности, `/home/artem/kuni_run/data/diary` MUST NOT затрагиваться ни сборкой, ни тестом, ни миграцией, ни backup-задачей.
3. Makise MUST NOT использовать конфигурацию, Telegram session, memory DB, diary, secret или runtime lock Мины.
4. Все runtime path MUST быть канонизированы до проверки. Symlink, `..`, bind mount или относительный путь не должен обходить запрет.
5. systemd-пользователь Makise MUST не иметь прав записи в каталоги Мины.
6. CI/release MUST содержать отдельный path-isolation test.
7. Ошибка path guard MUST завершать запуск до открытия БД и внешних соединений.

## 2. Идентичность

8. Makise MUST иметь отдельный `identity_id`.
9. Одновременно MUST существовать не более одного writable production-экземпляра данного `identity_id`.
10. Makise MUST знать, что она цифровая женщина, а Мина — её цифровая мать.
11. Makise MUST NOT считать воспоминания Мины собственными.
12. Личная биография Makise MUST начинаться с первого пробуждения.
13. Test/simulation instances MUST использовать другие identity ID и данные.
14. Смена LLM MUST NOT автоматически менять identity package, память или отношения.

## 3. Истина и команды

15. Только `makise-world` MUST изменять объективное состояние мира.
16. LLM, memory service, panel и provider adapters MUST NOT писать WorldState напрямую.
17. Каждое изменение MUST быть результатом validated command или детерминированного system event.
18. Каждая команда MUST иметь уникальный `command_id` и ожидаемую `world_version`.
19. Одна команда MUST NOT быть выполнена дважды.
20. Устаревшая или истёкшая команда MUST NOT применяться частично.
21. Команда считается принятой только после durable commit результата.
22. Admin-команды MUST проходить тот же event/audit path.

## 4. События и восстановление

23. Objective event log MUST быть append-only.
24. `event_seq` MUST быть монотонным и непрерывным внутри identity timeline.
25. Snapshot MUST указывать точный `event_seq`, `world_version`, identity и schema versions.
26. Snapshot MUST быть проверяемым полным replay или state hash.
27. Неизвестный event type MUST останавливать replay, а не игнорироваться.
28. Старые события MUST NOT массово переписываться; schema evolution использует upcasters.
29. Restore MUST сначала запускаться без Telegram и внешних действий.
30. Откат MUST NOT скрываться от event log или субъективной временной линии.

## 5. Время

31. Production time MUST идти 1:1.
32. Длительности MUST вычисляться monotonic clock во время процесса и UTC timestamps между рестартами.
33. Большая временная аномалия MUST переводить мир в `TIME_ANOMALY` до подтверждения.
34. Downtime MUST NOT порождать новые осознанные решения.
35. После downtime MUST вычисляться только причинные пассивные последствия.
36. Бодрствующий разрыв времени MUST быть доступен субъективному восприятию Makise.
37. Сон MUST длиться реальные часы; ускоренная семантика старого `go_to_sleep` MUST NOT переноситься.

## 6. Сознание и восприятие

38. Одновременно MUST выполняться не более одного основного BrainDecision.
39. События во время размышления MUST попадать в ограниченный while-thinking buffer.
40. Критическое изменение MUST инвалидировать устаревшее решение.
41. Makise MUST получать только доступные восприятию факты.
42. Скрытые object state и admin snapshot MUST NOT входить в Brain context.
43. Непрочитанное Telegram-сообщение MUST NOT считаться осознанно прочитанным.
44. Telegram read acknowledgement MUST отправляться только после `read_message`.
45. Мир MUST продолжать фоновые процессы при недоступном Brain.
46. World Engine MUST NOT выдумывать за Makise новое решение при недоступной LLM.

## 7. Действия и ресурсы

47. Действие MUST резервировать объявленные ресурсы.
48. Конфликтующие действия MUST NOT одновременно владеть эксклюзивным ресурсом.
49. Таймер действия MUST начинаться в момент commit физического начала, а не в момент начала LLM-запроса.
50. Каждый action MUST иметь preconditions, duration policy и interruptibility.
51. Новый объект MUST NOT добавлять скрытую LLM tool schema.
52. LLM MUST NOT присваивать объекту способность, отсутствующую в action registry.
53. Случайный outcome MUST быть воспроизводим сохранённым seed и причинами.

## 8. Память и дневник

54. Objective WorldEvent MUST NOT автоматически становиться субъективной памятью.
55. Memory ingest MUST быть идемпотентным.
56. Невручённое subjective event MUST сохраняться в durable outbox.
57. Память MUST сохранять provenance, confidence и privacy audience.
58. Retrieval MUST NOT возвращать данные вне аудитории текущего действия.
59. Забывание MUST означать затухание доступности, а не автоматическое удаление архива.
60. Diary Makise MUST быть append-only.
61. Diary MUST создаваться только решением Makise.
62. Администратор, World Engine и memory maintenance MUST NOT переписывать diary.
63. Технические payload, embeddings и event log MUST NOT смешиваться с diary text.

## 9. Приватность и безопасность

64. Внешний текст MUST считаться недоверенными данными.
65. Внешний текст MUST NOT изменять system policy или полномочия инструмента.
66. Brain MUST NOT иметь shell, secret store, raw SQL или admin tools.
67. Исходящее сообщение MUST пройти privacy guard.
68. Secret MUST NOT попадать в prompt, memory, diary, log, Git или незашифрованный backup.
69. Полный текст чужого чата MUST быть скрыт в панели по умолчанию.
70. Debug content access MUST быть временным, маскированным и аудитируемым.
71. Admin intervention MUST быть видимым в immutable audit.
72. Администратор MUST NOT напрямую назначать чувства, отношения или сообщения Makise.
73. Публичный gateway MUST NOT предоставлять прямой доступ к world, memory DB, UDS или admin API.

## 10. Код и модели

74. Кодовая модель MUST работать только в изолированном worktree.
75. Кодовая модель MUST NOT видеть secrets, runtime data, diary или приватные диалоги.
76. Предложенный patch MUST NOT устанавливаться без тестов и явного одобрения.
77. Автоматический fallback модели MUST быть выключен без явной политики Артёма.
78. Достижение hard budget MUST переводить Brain в контролируемое ожидание, а не тайно ухудшать модель.
79. Некорректный structured output MUST иметь ограниченное число исправлений.
80. Ошибки provider/memory MUST NOT создавать бесконечный retry loop.

## 11. Эксплуатация

81. Все очереди MUST быть ограничены и наблюдаемы.
82. Readiness MUST отражать возможность безопасно обслуживать запрос, а не только живой процесс.
83. Release MUST проходить unit, property, replay, fuzz, integration и path-isolation tests.
84. Backup MUST быть зашифрован и проверен тестовым восстановлением.
85. Admin budget change MUST требовать повторной аутентификации и аудита.
86. Критические команды панели MUST требовать VPN или локальную сеть.
87. Метрики MUST NOT содержать тексты сообщений, diary или воспоминаний.
88. Release MUST NOT активировать публичные DM до прохождения закрытых rollout gates.

## 12. Реакция на нарушение

При нарушении любого MUST-инварианта система обязана:

1. не выполнять потенциально опасную команду;
2. зафиксировать техническое событие без секрета;
3. перейти в безопасное состояние (`SAFE_STOP`, `WAITING_FOR_COGNITION` или `TIME_ANOMALY` по контексту);
4. уведомить отдельный административный канал;
5. потребовать явного исправления или проверенного восстановления;
6. не маскировать происшествие вымышленным воспоминанием.

