---
title: "ReactiveCell<T>"
sidebar:
  label: ReactiveCell
  order: 10
---

`ReactiveCell<T>` предоставляет одинаковое API чтения, записи и подписок для
поля, записи карты, динамического пути и значения в памяти. Компонент может
принять ячейку, не зная, из какой структуры, карты или store получено значение.

<!-- shown: four ways to reach a cell -->
```rust
let width = state.sidebar_width().cell();
let cpu_column = state.widths().entry_cell("cpu".to_string());
let by_path = store.kv().cell("dragging", 0u64)?;
let loose = ReactiveCell::new_volatile(0u64);

let mut columns: HashMap<String, ReactiveCell<u64>> = HashMap::new();
columns.insert("sidebar".to_string(), width);
columns.insert("cpu".to_string(), cpu_column);
columns.insert("dragging".to_string(), by_path);
columns.insert("loose".to_string(), loose);
```
<!-- /shown -->

Первые три примера записывают изменения в store. `ReactiveCell::new_volatile`
хранит значение только в памяти. Ячейка от поля с `#[amestate(volatile)]`
тоже не сохраняется между запусками.

`kv.cell` создаёт ячейку по пути и default без объявления структуры.
Он читает существующие данные, а не регистрирует тип. Запрос несовместимого
типа по тому же пути вызовет ошибку декодирования; совместимые типы
отдельным реестром не различаются.
[Kv](/amethystate/ru/primitives/kv/#один-путь--один-тип).

## Чтение

```rust
fn get(&self) -> Option<T>
```

`get` возвращает `None`, если записи карты нет или источник ячейки уже удалён.
В отличие от [`Field`](/amethystate/ru/primitives/field/), ячейка не всегда
может вернуть значение.

Чтение идёт из кэша ячейки, без обращения к хранилищу, поэтому `get` можно
вызывать в цикле отрисовки.

<!-- shown: reading, writing and watching a cell -->
```rust
let current = cell.get();

let _sub = cell.subscribe(|width| println!("width -> {width:?}"));

cell.set(200)?;
cell.update(|width| width + 10)?;
cell.modify(|width| *width += 10)?;
```
<!-- /shown -->

## Записи ложатся туда, где живёт значение

Ячейка передаёт запись своему источнику. Сохраняемое поле обновляется в store,
а volatile-источник — в памяти.

Кэш ячейки обновляется после того, как источник примет изменение. Если запись
отклонена, `get` продолжает возвращать прежнее значение. Принятая запись при этом
ещё не обязательно сохранена на диск.

## Ячейка на запись карты

<!-- shown: a cell onto a map entry -->
```rust
state.widths().insert("cpu".to_string(), &120)?;

let cpu = state.widths().entry_cell("cpu".to_string());
let absent = state.widths().entry_cell("gpu".to_string());

assert_eq!(cpu.get(), Some(120));
assert_eq!(absent.get(), None);

state.widths().remove("cpu")?;

assert_eq!(cpu.get(), None);
assert!(cpu.set(80).is_err());
```
<!-- /shown -->

Ячейка записи пуста, пока ключ отсутствует, в том числе после удаления.
`set`, `update` и `modify` тогда возвращают `WriteValue::Absent` с путём записи.
Верните ключ через карту, прежде чем писать через его ячейку.

## Что ячейка держит живым

Есть два варианта времени жизни:

- `cell()` и `entry_cell()` хранят **слабую ссылку** и не продлевают жизнь
  источника. Когда удалён последний владеющий хендл, чтение возвращает `None`,
  а запись — `WriteValue::SourceGone`.
- `into_cell()` и `into_entry_cell()` **сохраняют источник живым**, удерживая
  владеющий хендл. Используйте их, если ячейка хранится в компоненте или может
  пережить исходную структуру состояния.

Геттер вроде `state.sidebar_width()` клонирует хендл, а не забирает поле
из структуры. Преобразование этого хендла через `into_cell()` добавляет
ячейку к владельцам того же источника.

<!-- shown: a view, and a cell that owns what feeds it -->
```rust
let view = state.sidebar_width().cell();
let owned = state.sidebar_width().into_cell();

drop(state);
```
<!-- /shown -->

Обе ячейки ссылаются на одно поле. Владеющая ячейка сохраняет его после
удаления структуры, поэтому слабая ячейка всё ещё читает значение. Когда
удалена и владеющая ячейка, слабая начинает возвращать `None`.
