---
title: Быстрый старт
sidebar:
  order: 3
---

В этом примере мы объявим настройки, откроем store и подпишемся на изменение
поля. Ссылки по ходу страницы ведут к подробному описанию каждого шага.

Прелюдия содержит нужные здесь типы и трейты:

```rust
use amethystate::prelude::*;
```

В том числе `StoreExt` и `StoreBackend`: эти трейты нужно импортировать для
вызова методов вроде `get` и `save_now`. В примерах ниже импорты выписаны явно,
чтобы было видно, из какого модуля взят каждый элемент.

## Объявите состояние

`#[amethystate]` создаёт сохраняемые реактивные поля на основе структуры.
`prefix` задаёт их расположение в store.

<!-- shown: declaring a state struct -->
```rust
use amethystate::amethystate;

#[amethystate(prefix = "network")]
pub struct NetworkState {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```
<!-- /shown -->

Значения по умолчанию, вложенные структуры, volatile-поля, политики
чтения, правила для значений и serde — в разделе
[Состояние](/amethystate/ru/state/defining-structs/).

## Создайте store

<!-- shown: opening a store you hold yourself -->
```rust
let store = StoreBuilder::new(settings)
    .disk(|d| d.debounce(Duration::from_millis(500)))
    .build()?;

let state = NetworkState::new_with(&store);
```
<!-- /shown -->

Передайте store в `new_with`, чтобы создать состояние. Можно также использовать
глобальный store или системный каталог данных приложения. Эти варианты и
закрытие store описаны в разделе [Открытие store](/amethystate/ru/store/opening/).

Задержки, повторные попытки и то, чего store не примет:
[Настройка store](/amethystate/ru/store/configuration/). В какой файл пишет
движок: [Установка](/amethystate/ru/getting-started/installation/).

## Читайте, пишите, подписывайтесь

<!-- shown: reading, writing and subscribing -->
```rust
println!("{}", state.host().get());

let _sub = state.port().subscribe(|port| {
    println!("port changed to {port}");
});

state.port().set(9090);
```
<!-- /shown -->

Запись обновляет память сразу, а store сохраняет её на диск позже.
В разделе [Подписки](/amethystate/ru/concepts/subscriptions/) разобраны потоки
вызова колбэков, фильтрация собственных записей и стоимость подписок.

Если нужно дождаться сохранения на диск:
[Durability](/amethystate/ru/concepts/durability/).

## Ключи, которые появляются в рантайме

Карта хранит каждую запись по отдельному пути. Записи можно добавлять
и отслеживать независимо друг от друга.

<!-- shown: a map whose keys are not known up front -->
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertThresholds {
    pub warning: u64,
    pub critical: u64,
}

#[amethystate(prefix = "sys")]
pub struct SystemSettings {
    #[amestate(default = {
        "cpu": AlertThresholds { warning: 70, critical: 90 },
        "mem": AlertThresholds { warning: 80, critical: 95 }
    })]
    pub limits: ReactiveMap<String, AlertThresholds>,
}
```
<!-- /shown -->

<!-- shown: working with a map -->
```rust
state.limits().insert(
    "gpu".to_string(),
    &AlertThresholds {
        warning: 60,
        critical: 85,
    },
)?;

let cpu = state.limits().get("cpu");

for (key, value) in state.limits().entries() {
    println!("{key}: {value:?}");
}

let _sub = state.limits().subscribe_any(|change| {
    println!("{change:?}");
});
```
<!-- /shown -->

`entries()` возвращает записи в порядке строковых ключей. Для нестроковых
ключей используйте `Id`: сортировка тоже идёт по строковому представлению,
поэтому ключи `Id<u16>` могут идти как `10, 100, 9`, а не по величине числа.

Для путей, выбранных во время работы, используйте [Kv](/amethystate/ru/primitives/kv/):
объявлять структуру не нужно. [ReactiveCell](/amethystate/ru/primitives/reactive-cell/)
предоставляет хендл для чтения, записи и отслеживания одного значения.

## Когда структура меняется

Поднимите `version` и объявите шаги между версиями — тогда библиотека перенесёт
данные, записанные старой сборкой. Поле переименовали или сменили ему тип, а
версию не подняли — это дрейф: о нём скажут, и запуск продолжится.

Открывайте store через `migrate` всегда, когда в бинарнике есть `#[migrate]`:
`build` не выполняет ни одного шага.

[Миграции](/amethystate/ru/migrations/overview/).

## Режим Persistent-only

Если нужны только загрузка и сохранение, используйте `mode = "persistent"`.
Поля останутся обычными значениями Rust, а сохранять структуру нужно явно.
Подписок в этом режиме нет, и изменения из других источников в структуру
не попадают. Это может быть удобно, если приложение уже управляет обновлениями
в своём цикле событий.

<!-- shown: a struct in persistent mode -->
```rust
#[amethystate(prefix = "kept", mode = "persistent")]
pub struct KeptSettings {
    #[amestate(default = "127.0.0.1".to_string())]
    pub host: String,

    #[amestate(default = 8080u16)]
    pub port: u16,
}
```
<!-- /shown -->

<!-- shown: writing a persistent struct -->
```rust
let mut state = KeptSettings::load_with(&store);

state.port = 9090;
state.save()?;

state.mutate(|d| {
    d.host = "0.0.0.0".to_string();
    d.port = 443;
})?;
```
<!-- /shown -->

`save_lazy` и `mutate_lazy` делают то же самое, но на диск сбрасывает
дебаунсер.

## Что ещё есть

- **Правила** — нормализация значений при чтении из хранилища и записи через
  поле: [Правила](/amethystate/ru/state/rules/).
- **Трассировка** — структурированные события: каждая запись помечена
  структурой, которая её сделала:
  [Наблюдаемость](/amethystate/ru/concepts/observability/).
- **guinea** — таймер, период которого следует за сохранённой настройкой:
  [guinea](/amethystate/ru/integrations/guinea/).
