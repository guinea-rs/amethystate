---
title: Обработка ошибок
sidebar:
  label: Ошибки
  order: 20
---

Операции возвращают перечисления ошибок для своего случая. Например,
`OpenStruct` описывает ошибки открытия структуры состояния, а `KvWrite` —
ошибки прямого доступа по пути. В `match` не нужно разбирать варианты,
которые в этом вызове невозможны.

Эти типы реализуют `std::error::Error`. Обрабатывайте отдельные варианты,
если можете восстановиться после ошибки, или передавайте её дальше через `?`.

## Разобрать отказ

<!-- shown: telling one refusal from another -->
```rust
let refused = match Panel::try_new_with(&store) {
    Ok(_) => return Ok(()),
    Err(why) => why,
};

let said = match refused {
    OpenStruct::WillNotRead { at, why } => format!("{at} holds something else: {why}"),
    OpenStruct::Taken(taken) => format!("{} already holds it", taken.held_by),
    OpenStruct::NotAPath(why) => format!("that is not a path: {why}"),
    OpenStruct::Store(disk) => format!("the store: {disk}"),
};
```
<!-- /shown -->

Ветки `_` нет: пример обрабатывает все варианты. Эти перечисления ошибок
исчерпывающие. Если в будущей версии добавится вариант, компилятор укажет
на `match`, которые нужно обновить.

Варианты содержат данные для диагностики. Например, `OpenStruct::Taken`
указывает запрошенные путь и структуру, а также конфликтующий путь и его
владельца. Выберите в `match` нужные поля, а остальные пропустите.

## Отдать его дальше

<!-- shown: letting the caller's own error type take it -->
```rust
fn with_anyhow(store: &amethystate::Store) -> anyhow::Result<()> {
    store.set(["ui", "width"], &800u32)?;
    Ok(())
}

fn with_a_box(store: &amethystate::Store) -> Result<(), Box<dyn Error + Send + Sync>> {
    store.set(["ui", "height"], &600u32)?;
    Ok(())
}
```
<!-- /shown -->

Ошибки реализуют `Send + Sync + 'static`, выводятся в одну строку через
`Display` и предоставляют исходную причину через `source()`. Их можно передать
в `anyhow`, `eyre` или `Box<dyn Error>` обычным `?`.

## Наборы

| набор | откуда |
| --- | --- |
| `OpenStruct` | `try_new`, `try_new_with`, `try_load`, `try_load_with`, `Open::try_new_with`, `AmeStateSlice::try_load_slice`, `new_with_id`, `new_with_id_under`, `Schema::open`, `Kv::cell` |
| `OpenStore` | `StoreBuilder::build`, `migrate`, `located` |
| `LoadMap` | `Kv::map` и собственный конструктор поля-карты |
| `ReadValue` | `Store::get` |
| `WriteValue` | `Store::set`, `Store::delete`, записи `Field::durable`, `ReactiveCell::set`/`update`/`modify`, `ReactiveMap::insert` |
| `KvWrite` | `Kv::get`, `Kv::set`, `Kv::remove` |
| `ScanKeys` | `Store::scan_keys`, `Store::scan_prefix`, `Kv::keys` |
| `Flush` | `save_now`, `close`, `flush_prefix` |
| `RunStep` | каждый метод `MigrationContext` и то, что отдаёт сам шаг |

Эти типы намеренно различаются. `WriteValue` может сообщить `Absent`
и `SourceGone`, а сырой вызов `Kv` — `Declared`. Отдельные перечисления
не заставляют вызывающий код разбирать невозможные случаи.

Обычные конструкторы вроде `new_with` паникуют при ошибке, которую `try_`-вариант
вернул бы результатом. `Field::set`, `update` и `modify` тоже не возвращают
ошибку: некодируемое значение вызывает панику, запись в закрытый store
игнорируется с предупреждением, а сбой сохранения сообщается наблюдателям.
Если нужен `Result`, используйте `Field::durable`.

У карты есть ошибки открытия для неверных путей записей и нечитаемых имён
ключей. Ошибки шага миграции различают несовместимую запись и сбой хранения,
чтобы шаг мог обработать их по-разному.

`Field::try_get` возвращает `Disagreement`: он описывает известную проблему
кэшированного поля, а не новую операцию хранения.
См. [Нечитаемые значения](/amethystate/ru/state/unreadable-values/).

## Отчёт никуда не делся

Варианты с ошибками хранилища сохраняют `error_stack::Report`, не только текст.
Это касается `Store`, `WillNotRead`, `WillNotEncode` и `TooDeep`.

<!-- shown: the report under a variant that named the failure -->
```rust
let refused = store.get::<u16>(["port"]).unwrap_err();

let amethystate::store::ReadValue::WillNotRead { at, why } = refused else {
    panic!("the bytes are there and they are not a u16")
};
```
<!-- /shown -->

Отчёт состоит из:

- **Контекстов:** категорий ошибки, добавленных при прохождении слоёв.
- **Приложений:** типизированных данных — путей, имён файлов, лимитов.

Получите отчёт через сопоставление внешней ошибки:

<!-- shown: getting at the report a set carries -->
```rust
fn what_the_store_said(why: LoadMap) -> StorageResult<()> {
    match why {
        LoadMap::EntryWillNotRead { why, .. } => Err(why.into_report()),
        other => panic!("an entry was expected to be at fault: {other}"),
    }
}
```
<!-- /shown -->

### Вершина называет операцию

<!-- shown: what a failure says it is -->
```rust
let refused = store.kv().map::<String, u64>("labels").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let context = report.current_context();
let sentence = report.to_string();
```
<!-- /shown -->

Здесь `context` равен `StorageError::Codec`, а `sentence` —
*the value could not be encoded or decoded*.

`current_context()` возвращает внешнюю категорию. `StorageError` обозначает
операцию, например `Write`, `Scan` или `Codec`; нижние фреймы сохраняют
причину конкретного движка.

В отличие от перечислений ошибок операций, `StorageError` помечен
`#[non_exhaustive]`. При сопоставлении нужна резервная ветка.

### Подробности приложены типами

В `amethystate::errors::facts` есть типизированные данные о ключах, префиксах
обхода, записях, файлах и размерах значений. Каждый тип хранит данные
и выводит их с понятной меткой.

`facts::all::<T, _>` получает все приложения типа, от внутренних к внешним:

<!-- shown: reaching the entry that failed -->
```rust
let refused = store.kv().map::<String, u64>("ports").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let entries: Vec<&Entry> = facts::all::<Entry, _>(&report).collect();
let prefixes: Vec<&Prefix> = facts::all::<Prefix, _>(&report).collect();
```
<!-- /shown -->

В `entries` находится `Entry("http")`, в `prefixes` — `Prefix("ports")`.
Они определяют запись карты, которую не удалось открыть; однострочное
сообщение не обязательно включает все приложения.

Запрос отсутствующего в отчёте типа ничего не возвращает:

<!-- shown: asking for a fact the report does not carry -->
```rust
let refused = store.kv().map::<String, u64>("ports").unwrap_err();
let report = what_the_store_said(refused).unwrap_err();

let key = facts::all::<Key, _>(&report).next();
```
<!-- /shown -->

`key` равен `None`: ошибка относится к записи под префиксом,
а отдельный ключ к ней не приложен.

Состав приложений зависит от данных, известных каждому слою. Это сведения
для диагностики, не фиксированная запись со всеми обязательными полями.
Они создаются только на пути ошибки.

Используйте отчёт для логов и диагностики, а внешние варианты ошибок —
для выбора действия восстановления.

## Как это печатается

`Display` (`{}`) выводит ошибку операции в одну строку. `Debug` (`{:?}`)
вложенного отчёта включает цепочку контекстов и типизированные данные:

<!-- shown: an entry that will not decode -->
```rust
store.set(["ports", "http"], &"text".to_string())?;

let undecodable = store.kv().map::<String, u64>("ports").unwrap_err();
```
<!-- /shown -->

<!-- printed: an entry that will not decode from book_errors -->
```
the entry at ports.http will not read back

the value could not be encoded or decoded
├╴as: u64
├╴value bytes: 5
├╴prefix: ports
├╴entry: http
│
├─▶ Erased codec error: wrong msgpack marker FixStr(4)
│
╰─▶ wrong msgpack marker FixStr(4)
```
<!-- /printed -->

<!-- shown: an entry whose name is not the map's key type -->
```rust
let wrong_key = store.kv().map::<Id<u16>, String>("ports").unwrap_err();
```
<!-- /shown -->

<!-- printed: an entry whose name is not the map's key type from book_errors -->
```
`http` under ports will not read as a Id<u16>
```
<!-- /printed -->

Этому варианту вложенный отчёт не нужен: он уже содержит карту,
запись и запрошенный тип ключа.

<!-- shown: a path that names nothing -->
```rust
let names_nothing = store.set(Vec::<String>::new(), &1u32).unwrap_err();
```
<!-- /shown -->

<!-- printed: a path that names nothing from book_errors -->
```
the write was given no path to land at: a path of no levels names the whole store - say `StorePath::root()` to mean it
```
<!-- /printed -->

<!-- shown: a path past the cap it was given -->
```rust
let shallow = StoreBuilder::new(settings)
    .limits(|l| l.key_depth(4))
    .build()?;

let too_deep = shallow.set(["a", "b", "c", "d", "e"], &1u32).unwrap_err();
```
<!-- /shown -->

<!-- printed: a path past the cap it was given from book_errors -->
```
a.b.c.d.e is deeper than this store reads back

deeper than this store reads back
├╴key: a.b.c.d.e
├╴levels: 5, and the limit is 4
├╴set by: limits(|l| l.key_depth(..))
╰╴what is stored here spends the same budget - this store reads 512 levels in all
```
<!-- /printed -->

Следующий отчёт отделяет короткое описание от подробностей: лимита
и фактического расхода. Такой вид удобен для диагностики — понятное резюме
и достаточно данных для расследования.

## Когда нужен именно `std::error::Error`

Сам `Report` не реализует `std::error::Error`, а перечисления ошибок операций
реализуют, поэтому обычный `?` работает. Если вы извлекли отчёт и передаёте
его через границу `E: Error`, преобразуйте его методом `into_error`:

<!-- shown: turning a report into a std error -->
```rust
let std_error = report.into_error();

let sentence = std_error.to_string();
let whole = format!("{std_error:?}");
```
<!-- /shown -->

Обёртка сохраняет отчёт: `sentence` не меняется, а в `whole` остаётся
`entry: http`. `as_error` даёт заимствованное представление ошибки,
не забирая отчёт.

`error_stack` реэкспортируется как `amethystate::error_stack`, а `Report`
доступен и через `amethystate::errors`. Так код использует ту же версию,
что и библиотека.
