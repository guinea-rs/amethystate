---
title: "Kv"
sidebar:
  order: 11
---

`Kv` даёт доступ по путям без объявления структуры состояния. Используйте его
для ключей, выбранных во время работы, или данных, которым не нужна схема.

<!-- shown: reading and writing without a schema -->
```rust
let kv = store.kv();

kv.set("theme", &"dark".to_string())?;
let theme = kv.get::<String>("theme")?;

let ui = kv.namespace("ui");
ui.set("width", &800u32)?;
let under_ui = ui.keys()?;

kv.remove("theme")?;
```
<!-- /shown -->

## Имя — это один уровень

Каждое имя в `Kv` — один сегмент пути. `kv.set("ui.width", ..)` записывает
единый ключ `ui.width`. Для записи `width` внутри `ui` используйте
`kv.namespace("ui").set("width", ..)`.

Точка внутри имени экранируется при хранении, поэтому эти вызовы обращаются
к разным ключам. Строка с точками не заменяет вызовы `namespace`.

`keys()` возвращает отсортированные пути на всех уровнях текущего пространства
имён. Для более узкого списка выберите вложенный namespace; корневой хендл
перечисляет весь store.

<!-- shown: what a listing covers -->
```rust
kv.set("theme", &"dark".to_string())?;
kv.namespace("ui").set("width", &800u32)?;
kv.namespace("ui").namespace("panel").set("left", &true)?;

let from_ui: Vec<String> = kv
    .namespace("ui")
    .keys()?
    .iter()
    .map(|key| key.to_string())
    .collect();

assert_eq!(from_ui, ["ui.panel.left", "ui.width"]);
```
<!-- /shown -->

Возвращённые пути включают namespace: `width` внутри `ui` выводится как
`ui.width`. Корневой ключ `theme` в список из `ui` не попадёт.

## Ячейки и карты

<!-- shown: a cell and a map with nothing declared -->
```rust
let width = kv.namespace("ui").cell("width", 800u32)?;
let flags = kv.map::<String, bool>("flags")?;

flags.insert("dark".to_string(), &true)?;
```
<!-- /shown -->

Эти методы возвращают [`ReactiveCell`](/amethystate/ru/primitives/reactive-cell/)
или `ReactiveMap` с такими же подписками, как у объявленных полей. Тип ячейки
выводится из её значения по умолчанию.

## Там, где уже живёт структура

`Kv` отклоняет запись по путям, занятым объявленным состоянием:

<!-- shown: writing where a struct lives -->
```rust
let said = match kv.namespace("network").set("port", &"8080".to_string()) {
    Err(KvWrite::Declared {
        at,
        declared_at,
        by,
    }) => {
        format!("{at} lies in {declared_at}, which {by} declared")
    }
    other => {
        other?;
        "the write went through".to_string()
    }
};

kv.namespace("networkish")
    .set("port", &"8080".to_string())?;
```
<!-- /shown -->

Ошибка выводится в одну строку:

<!-- printed: writing where a struct lives from book_kv -->
```
network.port is declared by Network
```
<!-- /printed -->

Можно также разобрать поля ошибки: `at` — путь записи, `declared_at` —
конфликтующий объявленный путь, `by` — структура, которая его объявила:

<!-- printed: what the caller can say about it from book_kv -->
```
network.port lies in network.port, which Network declared
```
<!-- /printed -->

Ключ `networkish` не конфликтует с `network`: владение определяется
сегментами пути, а не совпадением начала строки.

Открытие `cell` или `map` по занятому пути возвращает `OpenStruct::Taken`,
а не ошибку прямой записи из примера выше.

Проверки не дают заменить объявленное поле несовместимым значением через сырой
доступ. Иначе поле продолжало бы показывать прежнее значение, а при следующем
запуске структура могла бы не открыться.

## Один путь — один тип

<!-- shown: asking for one path as two types -->
```rust
let ui = kv.namespace("ui");

let _width = ui.cell("width", 800u32)?;
let refused = ui.cell("width", String::new());
```
<!-- /shown -->

У `Kv` нет реестра типов. Создание ячейки читает сохранённое значение;
если другая ячейка запросит несовместимый тип, декодирование завершится ошибкой.

Проверка работает и после перезапуска, поскольку значение остаётся в store.
Если путь пуст, первая ячейка записывает свой default. Прямой `set` работает
иначе: он может заменить значение данными другого типа.

```rust
kv.set("thing", &1u32)?;
kv.set("thing", &"now a string".to_string())?;   // replaces it
kv.get::<u32>("thing")?;                          // Err, at the read
```

После такой замены `get` со старым типом вернёт ошибку декодирования.

## Чем приходится платить

Версии, миграции и поиск дрейфа доступны объявленным структурам. Если форму
данных нужно версионировать, объявите структуру вместо набора сырых ключей.

## Как это выглядит в файле

В redb и SQLite путь `Kv` хранится как обычный ключ.

В документах JSON, TOML и RON объявленные структуры образуют вложенные деревья.
Необъявленные пути записываются рядом с ними **целиком как ключи**, без разбивки
на поля документа:

<!-- shown: a declared struct and a Kv write, side by side in one file -->
```rust
store
    .kv()
    .namespace("plugins")
    .namespace("left")
    .set("width", &240u32)?;
```
<!-- /shown -->

<!-- printed: the file a Kv write leaves from book_kv -->
```
{
  "chrome": {
    "width": 800
  },
  "plugins.left.width": 240
}
```
<!-- /printed -->
