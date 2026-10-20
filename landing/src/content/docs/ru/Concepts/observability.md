---
title: Наблюдаемость
sidebar:
  order: 19
---

`amethystate` использует [`tracing`](https://docs.rs/tracing) для структурированных
логов. События имеют таргет `amethystate`, который можно фильтровать отдельно
от логов приложения.

## Как включить трассировку

Трассы записей полей и вызовов подписчиков используют `TRACE`.
Включите этот уровень для таргета `amethystate` в подписчике tracing:

```rust
tracing_subscriber::fmt()
    .with_env_filter("amethystate=trace")
    .init();
```

Фильтр можно также задать через `RUST_LOG`:

```sh
RUST_LOG=amethystate=trace cargo run
```

## Что попадает в лог

### Записи в поля

Вызов `field.set()` создаёт трассу попытки записи:

```
TRACE amethystate: field write path=network.port source=NetworkState
TRACE amethystate: field write path=network.port source=external
```

| Поле | Значение |
| --- | --- |
| `path` | Путь хранения, например `network.port`. |
| `source` | Короткое имя структуры-владельца хендла либо `external` для отдельного хендла: форка или поля из `field_with_path`. |

### Срабатывания подписок

Каждый вызов колбэка создаёт трассу подписки:

```
TRACE amethystate: signal emit → subscription fire subscription_id=0 name=PortWatcher location=src/main.rs:42
TRACE amethystate: map signal emit → any subscription fire subscription_id=1 name=None location=src/server.rs:88
```

| Поле | Значение |
| --- | --- |
| `subscription_id` | Возрастающий ID подписки внутри её сигнала. |
| `name` | Метка из `.named()` либо `None`. |
| `location` | Файл и строка вызова подписки. |

## Как помечать подписки

Место вызова сохраняется через `#[track_caller]`. Добавьте `.named()`, если
строка в обобщённом коде или фреймворке недостаточно ясно определяет подписку:

```rust
let _sub = state.port()
    .subscribe(|p| do_something(p))
    .named("PortWatcher");

// ReactiveMap works the same way
let _sub = state.limits()
    .subscribe_any(|change| handle(change))
    .named("LimitsWatcher");
```

Метка появляется в трассах этой подписки.

## Как читать вывод

Запись и последующие вызовы двух колбэков выглядят так:

```
TRACE amethystate: field write path=network.port source=NetworkState
TRACE amethystate: signal emit → subscription fire subscription_id=0 name=PortWatcher location=src/ui.rs:55
TRACE amethystate: signal emit → subscription fire subscription_id=1 name=None location=src/logger.rs:12
```

`source=external` **не означает** внешний процесс. Это локальный `set()`
через хендл без структуры-владельца. Правки файла и записи миграций
не проходят через `Field::set`, поэтому трассу `field write` не создают.
