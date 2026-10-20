---
title: Ручные миграции
sidebar:
  order: 22
---

Ручные шаги нужны для прямой работы с ключами, чтения другого префикса
или явного удаления записей карты. Они выполняются тем же движком миграций,
что и шаги с `#[migrate]`.

## Точка входа

Зарегистрируйте ручные шаги через `.migrations()` билдера:

<!-- shown: registering steps by hand -->
```rust
let (store, report) = StoreBuilder::new(app)
    .migrations(|m| {
        m.for_prefix("net")
            .step(1, "move off the privileged port", |ctx| {
                ctx.set("port", &8080u16)
            });
    })
    .migrate()?;
```
<!-- /shown -->

Завершите билдер вызовом `migrate`: он выполнит эти шаги и функции
с `#[migrate]`, включённые в бинарник, и вернёт отчёт. После `.migrations()`
метод `build` недоступен, чтобы зарегистрированные шаги нельзя было молча
пропустить. Другие настройки store задайте до этого вызова.

Шаги одной линии идут по версиям. Зависимости между префиксами определяются
при чтении — см. [Чтение из чужого префикса](#чтение-из-чужого-префикса).

## Объявление шага

```rust
m.for_node::<Profile>()
    .step(2, "split full name", |ctx| {
        let full_name = ctx.get::<String>("full_name")?.unwrap_or_default();
        let mut parts = full_name.splitn(2, ' ');
        ctx.set("first_name", &parts.next().unwrap_or_default().to_string())?;
        ctx.set("last_name", &parts.next().unwrap_or_default().to_string())?;
        ctx.delete("full_name")?;
        Ok(())
    });
```

`for_node::<T>()` выбирает линию по префиксу и `id` структуры.
`for_prefix("net")` выбирает безымянную линию префикса,
`for_named("ui", "panels")` — именованную.

`.step(version, description, closure)` переводит `version - 1` в `version`.
Для каждой целевой версии линии допускается один шаг. Если ручной шаг
дублирует сгенерированный, `MigrationError::StepTwice` возникает до выполнения
шагов этой линии.

Сгенерированный шаг не может менять префикс или `id`: макрос отклоняет такой
переход. Для переноса данных между расположениями нужны явные глобальные
чтения и записи.

## API контекста

`ctx` шага читает и пишет сохранённые ключи напрямую. Пути, кроме глобальных,
отсчитываются от префикса линии.

### Базовые операции

| Метод | Описание |
| --- | --- |
| `ctx.get::<T>(key)` | Прочитать значение как `Result<Option<T>>`. |
| `ctx.set(key, value)` | Записать значение. |
| `ctx.delete(key)` | Удалить ключ. |
| `ctx.rename(from, to)` | Скопировать в новый путь и удалить источник; ничего не делать, если источника нет. |

### Составные операции

Эти методы объединяют чтение, преобразование и запись:

| Метод | Описание |
| --- | --- |
| `ctx.transform::<TOld, TNew>(key, f)` | Заменить существующее значение на `f(value)`; пропустить отсутствующее. |
| `ctx.merge::<T1, T2, TNew>((k1, k2), into, f)` | Объединить два значения в целевом пути и удалить исходные; пропустить, если любого источника нет. |
| `ctx.split::<TOld, T1, T2>(from, (k1, k2), f)` | Разделить значение по двум путям и удалить источник; пропустить отсутствующий. |

Например:

```rust
// multiply a value in place
ctx.transform::<u16, u16>("sample_rate", |r| Ok(r.saturating_mul(10)))?;

// combine first_name + last_name into display_name, remove originals
ctx.merge::<String, String, String>(
    ("first_name", "last_name"),
    "display_name",
    |f, l| Ok(format!("{f} {l}")),
)?;

// split "host:port" into two separate keys
ctx.split::<String, String, u16>(
    "address",
    ("host", "port"),
    |s| {
        let (h, p) = s.split_once(':').unwrap();
        Ok((h.to_string(), p.parse()?))
    },
)?;
```

### Коллекции

| Метод | Описание |
| --- | --- |
| `ctx.scan_map::<K, V>(key)` | Прочитать записи под `prefix.key` как `IndexMap<K, V>` в порядке карты. |

Так можно мигрировать `ReactiveMap` без создания `AmeData`:

```rust
let old_routes = ctx.scan_map::<String, String>("routes")?;
ctx.delete_prefix("routes")?;

for (name, url) in &old_routes {
    ctx.set(&format!("endpoints.{name}"), url)?;
}
```

`delete_prefix` удаляет старое поддерево карты одним вызовом.

`scan_map` должен прочитать каждый ключ и значение. Нечитаемая запись
завершает шаг ошибкой с откатом транзакции, а не молча теряется при преобразовании.

### Глобальный доступ

| Метод | Описание |
| --- | --- |
| `ctx.global_get::<T>(path)` | Прочитать полный путь в любом месте store. |
| `ctx.global_set(path, value)` | Записать полный путь в любом месте store. |

Эти методы не добавляют префикс текущей линии. Они нужны для чтения или переноса
данных за его пределами:

```rust
let plan = ctx.global_get::<String>("identity.plan")?.unwrap();
```

### Ограничение области

`ctx.scoped(sub_prefix)` создаёт контекст под текущим префиксом.
Он также используется внутри `ctx.nested()`.

## Значения от приложения

Функция `#[migrate]` не захватывает локальные переменные, а регистрируемое
замыкание должно быть `'static`. Передавайте зависимости, например таблицы
соответствий, через билдер. Хранится одно значение на тип, и шаг получает его
по этому типу:

<!-- shown: a value the application provides -->
```rust
struct LegacyDefaults {
    port: u16,
}

let (store, report) = StoreBuilder::new(app)
    .provide(LegacyDefaults { port: 8080 })
    .migrations(|m| {
        m.for_prefix("net")
            .step(1, "fill in the port the old build assumed", |ctx| {
                let port = ctx.require::<LegacyDefaults>()?.port;
                ctx.set("port", &port)
            });
    })
    .migrate()?;
```
<!-- /shown -->

`ctx.provided::<T>()` возвращает `None`, если зависимости нет.
`ctx.require::<T>()` вместо этого возвращает `RunStep::NothingProvided`
с запрошенным типом и списком доступных.

## Чтение из чужого префикса

Когда шаг вызывает `ctx.global_get` для другого префикса, движок сначала
мигрирует тот префикс в той же транзакции. Отдельный список порядка не нужен:

```rust
m.for_node::<Profile>()
    .step(2, "snapshot plan from identity", |ctx| {
        let plan = ctx
            .global_get::<String>("complex_identity.plan")?
            .expect("reading it is what migrates it");
        ctx.set("plan_snapshot", &plan)?;
        Ok(())
    });
```

Поэтому чтение получает уже мигрировавшие данные. Если `Workspace` читает
`Profile`, а `Profile` — `Identity`, чтения зависимостей сначала завершают
`Identity`, затем `Profile`, после чего продолжается `Workspace`.
Порядок регистрации на это не влияет.

Чтение префикса, миграция которого уже выполняется, создаёт цикл зависимостей.
Ошибка показывает весь цикл, чтобы можно было убрать круговую зависимость.

## Как достать контекст из #[migrate]

Функция `#[migrate]` тоже может принять `MigrationContext` вторым аргументом.
Ей доступны все перечисленные методы. О сгенерированной очистке и случаях,
когда нужно явное удаление, см.
[Объявление шагов](/amethystate/ru/migrations/defining-steps/#что-шаг-вычищает-сам).

## Падение и откат

Движок объединяет миграцию и префиксы, к которым она обращается, в одну
транзакционную **компоненту**. Ошибка шага откатывает эту компоненту,
но не независимые. Другие компоненты могут уже быть зафиксированы или
успешно мигрировать дальше.

Store после этого не открывается: `OpenStore::Migrating` содержит полный отчёт.
Так приложение не получает данные, не соответствующие запрошенной схеме:

<!-- shown: a step that fails refuses the open -->
```rust
let refused = StoreBuilder::new(app)
    .migrations(|m| {
        m.for_prefix("net").step(1, "turns the data down", |_| {
            Err(MigrationError::Custom("this data is not ours".into()).into())
        });
    })
    .migrate();

let Err(OpenStore::Migrating {
    report: Some(report),
    ..
}) = refused
else {
    panic!("a failed step let the store open");
};
```
<!-- /shown -->

Отчёт указывает неудавшиеся префиксы и причины. Их компоненты сохраняют
прежние версии, успешные — уже обновлены.

Если приложение может работать без сохранённого состояния, `or_in_memory()`
перед `migrate` даст новый store в памяти. `build` пропускает шаги, но
не исправляет данные: используйте его только при явной поддержке старой
формы данных вашим кодом.

<!-- shown: reading what failed, then opening without the steps -->
```rust
for component in &report.components {
    if let ComponentOutcome::Failed { error, .. } = &component.outcome {
        eprintln!("{:?} was left as it was: {error:?}", component.prefixes);
    }
}

let store = StoreBuilder::new(app).build()?;
```
<!-- /shown -->

Неудавшаяся компонента перечисляет участвовавшие префиксы. Ошибка шага
содержит префикс, целевую версию и файл store.

### Что остаётся на диске после остановки

Ошибка шага откатывает компоненту до коммита. Остановка процесса во время
фиксации текстовых файлов — другой случай: данные и метаданные лежат
отдельно и не заменяются одновременно.

Текстовый движок сначала пишет метаданные с отметкой о начатом коммите,
потом данные, затем окончательные метаданные. При следующем открытии:

- если данные совпадают с новым состоянием, коммит метаданных завершается;
- если со старым — отметка убирается и миграция может выполниться повторно;
- если ни с одним — открытие возвращает ошибку, не пытаясь угадать схему.
