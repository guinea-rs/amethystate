---
title: Как открыть структуру по-своему
sidebar:
  label: Открытие структуры
  order: 5
---

Каждая структура со своим местом реализует два трейта. `Schema` открывает её
ровно так, как она объявлена: поля прочитаны, их правила пройдены, место
занято. `Open` — то, чем её открывает остальная программа, и если структура не
сказала иного, это `Schema` и ничего сверх. Код, который открывает структуры,
не зная их по имени, просит `Open`:

<!-- shown: opening any struct that has a place of its own -->
```rust
fn opened<S: Open>(store: &Store) -> S {
    S::new_with(store)
}
```
<!-- /shown -->

## `open = manual`

`open = manual` нужен структуре, которую мало открыть по объявлению: значение
надо вывести из машины, на которой она работает, или сверить с тем, что знает
приложение. Тогда макрос `Open` не пишет, его пишет автор структуры, и
начинает со `Schema`:

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

Автор пишет `try_new_with` — дверь, которая отвечает `Result`; `new_with`,
который паникует с тем же ответом, даёт сам трейт. Всё нужное берут из
`store.context()` — оттуда же, откуда объявленные
[правила](/amethystate/ru/state/rules/), — а `require` паникует, называя
значение, которого никто не передал: store без него собран неправильно
одинаково при каждом запуске.

## Инвариант между полями

Правило поля видит одно значение и не видит соседей, а структура — место, где
лежат поля, а не значение со своим правилом. Поэтому инвариант между полями
идёт сюда. Написанный руками `Open` видит все поля разом, каждое уже прошло
своё правило, и поправляет их так же, как поправило бы правило:

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

Это происходит один раз, там, где структуру открывают. Запись в `min` после
этого проходит только правило самого `min`, а про `max` оно не знает. Два поля,
которые пишут из двух потоков, — ограничение, о котором говорит страница
[Долговечность](/amethystate/ru/concepts/durability/#конкурентный-доступ).

## Единственный вход

Написанный руками `Open` — единственный вход. Через него идут и `new_with` с
`try_new_with` у структуры, и `load_with` с `try_load_with` у
persistent-структуры, и `AmeStateSlice`, так что мимо него структуру случайно
не откроет никто. Поэтому же сам impl начинает с `<Self as Schema>::open`, а не
с `Self::try_new_with`: второй и есть этот самый `Open` и позвал бы сам себя. А у
структуры с `open = manual` нет ни `new()`, ни `load()` через глобальный store:
дотянуться до него — ошибка компиляции, а не то, что должно поймать ревью.

`open` пишут только у структуры со своим местом. Вложенную открывает та, что её
держит, поэтому на ней макрос `open` не примет.
