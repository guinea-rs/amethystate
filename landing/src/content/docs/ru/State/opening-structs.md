---
title: Как открыть структуру по-своему
sidebar:
  label: Открытие структуры
  order: 5
---

Структура состояния со своим расположением в store реализует два трейта:

- `Schema` загружает объявленные поля, выполняет их правила и занимает пути.
- `Open` — интерфейс создания структуры для кода приложения. Сгенерированная
  реализация вызывает `Schema`, но её можно заменить собственной.

В обобщённом коде, который создаёт структуры состояния, используйте ограничение `Open`:

<!-- shown: opening any struct that has a place of its own -->
```rust
fn opened<S: Open>(store: &Store) -> S {
    S::new_with(store)
}
```
<!-- /shown -->

## `open = manual`

Используйте `open = manual`, если при создании нужна дополнительная работа:
например, подстроить настройки под текущий компьютер или проверить их по данным
приложения. Макрос перестанет генерировать `Open`. Реализуйте его самостоятельно,
сначала вызвав `Schema` для обычной загрузки:

<!-- shown: a struct that opens its own way -->
```rust
#[amethystate(prefix = "agent", open = manual)]
pub struct AgentSettings {
    #[amestate(default = 0u32)]
    pub workers: u32,
}

impl Open for AgentSettings {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let machine = store.context().require::<Machine>();
        let settings = <Self as Schema>::open(store)?;

        if settings.workers().get() == 0 {
            settings.workers().set(machine.cores);
        }

        Ok(settings)
    }
}
```
<!-- /shown -->

Реализуйте `try_new_with`, возвращающий `Result`. Трейт предоставляет
`new_with`, который паникует при ошибке. Зависимости берите из `store.context()`,
как в [правилах](/amethystate/ru/state/rules/). `require` паникует, если значение
нужного типа не было передано при создании store.

## Инвариант между полями

Правило поля видит только его значение. Проверки нескольких полей размещайте
в `Open`: здесь доступны все поля, и их отдельные правила уже выполнены:

<!-- shown: an invariant between fields, checked where the struct is opened -->
```rust
#[amethystate(prefix = "window", open = manual)]
pub struct Window {
    #[amestate(default = 400u32)]
    pub min: u32,

    #[amestate(default = 1600u32)]
    pub max: u32,
}

impl Open for Window {
    fn try_new_with(store: &Store) -> Result<Self, OpenStruct> {
        let window = <Self as Schema>::open(store)?;

        if window.min().get() > window.max().get() {
            window.max().set(window.min().get());
        }

        Ok(window)
    }
}
```
<!-- /shown -->

Эта проверка выполняется только при открытии структуры. Последующая запись
в `min` проходит правило поля, но не эту общую проверку. Поддерживать инвариант
при дальнейших изменениях должно приложение. См. также
[Конкурентный доступ](/amethystate/ru/concepts/durability/#конкурентный-доступ).

## Единственный вход

Сгенерированные конструкторы вызывают ваш `Open`: `new_with` и `try_new_with`,
`load_with` и `try_load_with` в persistent-режиме, а также загрузка через
`AmeStateSlice`.

Внутри реализации вызывайте `<Self as Schema>::open`, а не `Self::try_new_with`:
второй вариант рекурсивно вызвал бы тот же `Open`. При `open = manual` макрос
также не создаёт конструкторы `new()` и `load()` для глобального store,
поэтому вызывающему коду нужно передать store явно.

`open` пишут только у структуры со своим местом. Вложенную открывает та, что её
держит, поэтому на ней макрос `open` не примет.
