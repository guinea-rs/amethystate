---
title: "ReactiveMap<K, V>"
sidebar:
  label: ReactiveMap
  order: 9
---

`ReactiveMap<K, V>` хранит записи по отдельным путям. Ключи можно добавлять
во время работы; подписки могут следить за всей картой или одной записью.
В отличие от `HashMap` в одном поле, записи можно изменять независимо.

## Чтение

<!-- shown: reading a map -->
```rust
let one: Option<u64> = widths.get("cpu");

if let Some(width) = widths.get("cpu") {
    println!("cpu is {width}");
}

let there = widths.contains_key("mem");
let how_many = widths.len();

for key in widths.keys() {
    println!("{key}");
}

for (key, value) in widths.entries() {
    println!("{key}: {value}");
}
```
<!-- /shown -->

`len` читает счётчик в памяти с учётом ещё не сохранённых изменений.
`keys` и `entries` обходят состояние карты в памяти; `keys` не десериализует
значения.

### Посмотреть, не забирая

`entries` клонирует возвращаемые значения. Чтобы заимствовать их, создайте
`view` и вызовите его метод `iter()`:

<!-- shown: looking at a map without cloning it -->
```rust
let held = widths.view();
let total: u64 = held.iter().map(|(_, width)| width).sum();

let mut widest = 0;
for (_, width) in &widths.view() {
    widest = widest.max(*width);
}
```
<!-- /shown -->

Цикл `for` сохраняет временный view до конца обхода. Если он нужен за пределами
этого выражения, привяжите его к переменной.

### Обход владеет версией

`view`, `entries` и `keys` фиксируют текущую версию карты. Обход не удерживает
блокировку, которая мешала бы записи. Карту можно менять во время чтения,
в том числе из того же потока:

<!-- shown: walking a map and writing to it -->
```rust
let mut walked = Vec::new();

for (key, width) in widths.entries() {
    widths.remove(&key)?;
    walked.push((key, width));
}

assert_eq!(walked, [("cpu".to_string(), 120), ("mem".to_string(), 80)]);
assert!(widths.is_empty());
```
<!-- /shown -->

Итератор всё равно посетит каждый исходный ключ: удаления меняют текущую карту,
а не версию, по которой идёт обход.

View, сохранённый на кадр, даёт согласованные чтения, даже если карта между ними
меняется. Он разделяет неизменённые данные с картой, а не копирует всю коллекцию,
но удерживает старые данные в памяти до своего удаления.

## Запись

<!-- shown: adding, changing and removing an entry -->
```rust
widths.insert("cpu".to_string(), &120)?;

widths.update("cpu", &200)?;
widths.modify("cpu", |width| *width += 10)?;

let absent = widths.update("gpu", &90);

widths.remove("cpu")?;
widths.clear()?;
```
<!-- /shown -->

`insert` добавляет или заменяет запись. `update` требует существующий ключ
и возвращает `Absent`, если его нет: событию `MapChange::Update` нужно
предыдущее значение. `modify` передаёт изменяемую ссылку, чтобы можно было
поправить значение без самостоятельной сборки замены.

`remove` отправляет один `MapChange::Remove` с ID записавшего хендла.
`clear` удаляет содержимое карты одной операцией store и отправляет одно
событие карты, а не по событию на запись.

Если новое значение зависит от наличия ключа, используйте `upsert`:

<!-- shown: writing an entry whether or not it is there -->
```rust
for page in ["home", "about", "home"] {
    hits.upsert(page, |seen| seen.map_or(1, |count| count + 1))?;
}

let now = hits
    .durable()
    .upsert("home", |seen| seen.map_or(1, |count| count + 1))?;
```
<!-- /shown -->

Замыкание получает `Some(previous)` или `None`. Результат сохраняется
и возвращается вызывающему коду. Ключ заимствуется: для изменения существующей
записи не нужно создавать новый `String`. Подписчики получают `Insert` для
нового ключа и `Update` для существующего.

Как и `modify`, это неатомарная операция чтения и записи: конкурентная запись
между этими шагами может быть перезаписана.

Для удаления части записей обойдите зафиксированную версию и удалите выбранные:

```rust
for (key, width) in widths.entries() {
    if width < 80 {
        widths.remove(&key)?;
    }
}
```

## Чем может быть ключ

`ReactiveMapKey` требует `AsRef<str>`. Поэтому строковые ключи не нужно
форматировать или разбирать при поиске.

Нестроковый ключ оберните в `Id`: он один раз форматирует значение и хранит
полученную строку:

```rust
let ports = store.kv().map::<Id<u16>, bool>("ports")?;

ports.insert(Id::new(8080), &true)?;
assert_eq!(ports.get("8080"), Some(true));
```

`Id::new` вычисляет строковое представление при создании ключа. Последующие
поиски используют его повторно, без нового форматирования и выделения памяти,
например для UUID. `get` возвращает это представление.

Пустая строка — тоже допустимый ключ: её можно читать, записывать и перечислять.

## В каком порядке приходят записи

Записи отсортированы по строковым ключам, как и при перечислении store:

<!-- shown: the order entries come back in -->
```rust
counts.insert("9".to_string(), &1)?;
counts.insert("10".to_string(), &1)?;
counts.insert("a.b".to_string(), &1)?;
counts.insert("a1b".to_string(), &1)?;

let order: Vec<String> = counts.keys().collect();
assert_eq!(order, ["10", "9", "a.b", "a1b"]);
```
<!-- /shown -->

Строковая сортировка отличается от числовой: `"10"` идёт раньше `"9"`, в том
числе у `Id<u16>`. Разделитель внутри имени сравнивается как его часть:
`a.b` идёт раньше `a1b`. Внутреннее экранирование путей этот порядок не меняет.

Порядок вставки карта не хранит. Если важен порядок колонок или элементов
списка, храните его отдельно, а карту используйте для поиска.

## Как подписаться на изменения

<!-- shown: hearing about a change -->
```rust
let _all = widths.subscribe_any(|change| {
    println!("{change:?}");
});

let _cpu = widths.subscribe_key("cpu".to_string(), |change| {
    println!("cpu: {change:?}");
});

widths.insert("cpu".to_string(), &120)?;
widths.insert("mem".to_string(), &80)?;
```
<!-- /shown -->

`subscribe_any` получает изменения любой записи, `subscribe_key` следит
за одним ключом. Оба возвращают guard, который управляет временем жизни подписки.

`subscription_with()` позволяет сочетать фильтры ключей, источника и поток
вместо колбэка. См. [Подписки](/amethystate/ru/concepts/subscriptions/).

### Что фильтрует `external`

На карте `.external()` отфильтровывает только собственные события **Update**
этого хендла. `Insert`, `Remove` и `Clear` всё равно приходят: появление
и удаление ключей меняет коллекцию, которую должен отобразить интерфейс,
независимо от того, кто сделал запись.

Поэтому `insert` нового ключа приходит в external-подписку, а `insert`
существующего отфильтровывается как обновление.
В [Подписках](/amethystate/ru/concepts/subscriptions/#чья-это-была-запись)
объясняется, как ID хендлов различают источники изменений.

## Одна запись как ячейка

`entry_cell(key)` позволяет передать одну запись как
[`ReactiveCell`](/amethystate/ru/primitives/reactive-cell/) коду,
которому не нужна сама карта.
