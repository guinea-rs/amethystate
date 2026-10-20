---
title: Поля
sidebar:
  order: 2
---

Без `#[amestate]` поле хранится под своим именем, а начальное значение берётся
из `Default::default()`. Атрибуты меняют эти настройки:

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    pub counter: u32, // no annotation — uses Default::default(), stored as "app.counter"

    #[amestate(default = 8080)]
    pub port: u16,
}
```

| Атрибут | Тип | Описание |
| --- | --- | --- |
| `path` | `&str` | Путь относительно префикса структуры. Точки разделяют уровни. |
| `default` | выражение | Начальное значение для пустого пути; без атрибута — `Default::default()`. |
| `nested` | флаг | Встроить другую структуру `#[amethystate]`. |
| `flatten` | флаг | Убрать собственный сегмент пути вложенного поля. |
| `volatile` | флаг | Хранить только в памяти, начиная с default на каждом запуске. |
| `with`, `serialize_with`, `deserialize_with` | путь | Собственные функции кодирования и декодирования с интерфейсом serde. |
| `on_unreadable` | вариант | Переопределить политику чтения структуры. На карте не допускается: используйте `unreadable_entries`. |
| `on_delete` | вариант | Политика удаления. При удалении уровня карты `UseDefault` восстанавливает объявленные записи, `Keep` оставляет карту пустой. Отдельная запись удаляется при обеих политиках. |
| `unreadable_entries` | вариант | Только на карте: `Skip` пропускает нечитаемые записи; `Refuse` по умолчанию возвращает ошибку с путём записи. |
| `rule` | `fn` | Нормализация чтения и записи. Defaults не обрабатываются. См. [Правила](/amethystate/ru/state/rules/). |

Атрибуты можно объединить в один `#[amestate(..)]` или указать несколькими аннотациями:

```rust
#[amestate(path = "panels.left.visible")]
#[amestate(default = true, on_delete = Keep)]
pub left_panel_visible: bool,
```

Повтор одного атрибута — ошибка компиляции, а не неявное переопределение.
Неизвестные параметры `amestate` тоже вызывают ошибку со списком допустимых.

## Атрибуты, которые не этого макроса

Прочие атрибуты переносятся на сгенерированное поле и геттер. Док-комментарии,
`#[allow]` и `#[deprecated]` сохраняют обычный смысл. Serde-атрибут без
соответствующего derive отклоняет rustc.

`#[cfg]` на отдельном поле не поддерживается: состав полей должен совпадать
в структуре, конструкторах, снапшотах и метаданных схемы. Ставьте `cfg`
на структуру целиком либо оставьте поле и выбирайте значение во время работы.

## Где лежит поле

`path` задаёт путь хранения поля. `rename_all` структуры определяет стиль имён
полей без явного пути. Точка в `path` создаёт уровень, а не входит в имя ключа:

<!-- shown: a struct that says where its fields go -->
```rust
#[amethystate(prefix = "net", rename_all = "camelCase")]
pub struct NetState {
    #[amestate(default = 8080u16)]
    pub listen_port: u16,

    #[amestate(path = "tls.enabled", default = false)]
    pub tls: bool,
}
```
<!-- /shown -->

Получаются пути `net.listenPort` и `net.tls.enabled`.

## Поле, чьи пути лежат на уровне хозяина

`flatten` на `nested`-поле размещает его дочерние поля на уровне родителя,
без собственного сегмента пути:

<!-- shown: a nested struct whose fields sit at their holder's level -->
```rust
#[amethystate(prefix = "editor")]
pub struct Editor {
    #[amestate(nested, flatten)]
    pub window: Window,
}
```
<!-- /shown -->

Получается `editor.width`, а не `editor.window.width`.

Пересечение путей вложенных или соседних полей вызывает ошибку компиляции.
Это касается и предков: поддерево `window` после flatten конфликтует
с соседним `path = "window.width"`, хотя полные пути не совпадают.

**Смена выпущенного `path`, добавление или удаление `flatten` требуют миграции.**
Иначе данные останутся по старому пути, а новый код прочитает пустой путь
и подставит default.

## Поле, которое лежит иначе

Чтобы изменить представление поля в хранилище, задайте функции кодирования
и декодирования:

```rust
#[amestate(with = since_the_epoch)]
pub opened: SystemTime,
```

`with = m` использует `m::serialize` и `m::deserialize`. Можно заменить только
одну функцию через `serialize_with` или `deserialize_with`; другая тогда
использует обычную реализацию serde типа. Убедитесь, что представления совместимы.

Сам тип поля всё равно должен реализовывать `Serialize` и `Deserialize`:
функции меняют представление на диске, а не требования к типу.
Чужой тип без serde оберните в newtype с реализациями этих трейтов.

Различие атрибутов состояния и сериализуемого значения описано в разделе
[Serde](/amethystate/ru/state/serde/).

## Поля volatile

Volatile-поля остаются в памяти и на каждом запуске получают default.
Используйте их для временного состояния интерфейса:

```rust
#[amethystate(prefix = "app")]
pub struct AppState {
    #[amestate(default = 8080)]
    pub port: u16,

    #[amestate(default = false, volatile)]
    pub loading: bool, // always starts as false, never written to disk
}
```

У volatile-поля тоже может быть [правило](/amethystate/ru/state/rules/).
Оно нормализует записи, хотя значение не сохраняется.
