---
title: Владение путями
sidebar:
  order: 16
---

`prefix` структуры вместе с путём поля определяет расположение значения
в store. Поэтому разные объявления могут обратиться к одному значению:

<!-- shown: two structs that want the same place -->
```rust
#[amethystate(prefix = "ui", version = 1)]
pub struct Ui {
    #[amestate(path = "panels.left.visible", default = true)]
    pub left_panel_visible: bool,
}

#[amethystate(prefix = "ui.panels", version = 1)]
pub struct Panels {
    #[amestate(path = "left.visible", default = true)]
    pub left_visible: bool,
}
```
<!-- /shown -->

Оба объявления задают `ui.panels.left.visible`, хотя по-разному разбивают
путь между префиксом и полем.

## Вторая получает отказ

Store регистрирует занятые пути. Открытие структуры с конфликтующим путём
возвращает ошибку:

<!-- shown: what the refusal looks like -->
```rust
let _ui = Ui::new_with(&store);

let refused = Panels::try_new_with(&store)
    .expect_err("`ui.panels.left.visible` is spelled by both of them");

let OpenStruct::Taken(taken) = &refused else {
    panic!("{refused}")
};

let Taken {
    at,
    wanted_by,
    held_at,
    held_by,
} = &**taken;

println!("{wanted_by} wants {at}, which {held_by} already holds at {held_at}");
```
<!-- /shown -->

`OpenStruct::Taken` описывает обе стороны: `at` и `wanted_by` — запрошенные
путь и структуру, `held_at` и `held_by` — существующее объявление.
Пути могут различаться, если один является предком другого.

Разбирайте поля ошибки, а не её текст. См.
[Ошибки](/amethystate/ru/concepts/errors/).

Проверка не даёт независимым структурам молча перезаписывать данные друг друга.

## Что считается пересечением

Пути конфликтуют, если совпадают или один является предком другого.
Порядок объявлений не важен; сравниваются сегменты, а не начало строки.

| два места | пересекаются |
| --- | --- |
| `ui` и `ui.panels` | да: предок и потомок |
| `ui.panels` и `ui` | да: тот же конфликт в обратном порядке |
| `ui.panels` и `ui.status` | нет |
| `ui` и `ui!x` | нет: разные имена сегментов |

Структуру нельзя открыть под листовым полем другой структуры. Например,
префикс `root.b` конфликтует с полем, сохранённым по пути `root.b`.

Соседние пути разрешены:

<!-- shown: a struct that sits right beside one and still opens -->
```rust
#[amethystate(prefix = "ui.panels.right", version = 1)]
pub struct RightPanel {
    #[amestate(default = true)]
    pub visible: bool,
}
```
<!-- /shown -->

`Ui` использует `ui.panels.left.visible`, а `RightPanel` —
`ui.panels.right.visible`. Ни один путь не является предком другого,
поэтому обе структуры открываются. Объявление всего `ui.panels`
конфликтовало бы с обеими.

## Кто чем владеет

- **Лист занимает свой путь и потомков.** Нельзя объявить дочерние данные
  под значением, которое кодируется целиком.
- **Карта занимает свой путь и поддерево.** Записи располагаются на следующем
  уровне; резервирование поддерева защищает и ключи, которых ещё нет.
- **Структура не занимает один лишь префикс.** Пути занимают её поля,
  а несвязанные соседние пути остаются свободны.

Например, это объявление:

<!-- shown: a map and a leaf under one prefix -->
```rust
#[amethystate(prefix = "ui")]
pub struct Panels {
    #[amestate(default = {})]
    pub open: ReactiveMap<String, u32>,

    #[amestate(default = "dark".to_string())]
    pub theme: String,
}
```
<!-- /shown -->

оставляет следующие пути свободными или занятыми:

```
ui                       ·  свободен — префикс никто не занимает
├─ theme                 ▪  объявил Panels; ниже ничего не бывает
├─ open                  ▪  объявил Panels, картой
│  ├─ left               ▪    запись, имя ей дали на ходу
│  └─ right              ▪    запись
└─ myplugin              ·  свободен — сюда может писать расширение
```

`ui.myplugin` доступен: структура не заняла весь `ui`.
`ui.open.myplugin` занят как потомок `ui.open`:

```
под префиксом структуры            под префиксом карты
  ui                                 ui.open
  ├─ theme       объявлено           ├─ left       запись
  ├─ open        объявлено           ├─ right      запись
  └─ myplugin    можно               └─ myplugin   отказ
```

Владение поддеревом не делает любой его путь допустимой записью карты.
`ui.open.left` — запись, а `ui.open.left.px` — нет: записи карты находятся
ровно на один уровень ниже неё.

**Вложенность значения не равна глубине пути.** Сложное значение записи
кодируется целиком по одному пути; его внутренние поля не становятся
отдельными путями store:

```
tree.roots.one           ▪  один ключ, одно хранимое значение
                            { name: "a", children: [ { … }, { … } ] }
                            что там внутри — дело кодека
```

## Занятое место переживает хендл

Удаление структуры не освобождает её пути. Регистрация принадлежит store
и сохраняется до его закрытия. Поэтому проверка конфликтов не зависит
от времени жизни локальных переменных.

## Как спросить, у кого место

<!-- shown: asking who claimed a place -->
```rust
let field = StorePath::parse_joined("ui.panels.left.visible")?;
let owner = store.places().declared_by(&field);

println!("{owner:?}");
```
<!-- /shown -->

Поиск требует точного занятого пути, а не префикса структуры.
В примере `Ui` зарегистрирована по `ui.panels.left.visible`, не по `ui`.
Результат — имя схемы, которое также используется в ошибке конфликта.
