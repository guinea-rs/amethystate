---
title: Нечитаемые значения
sidebar:
  order: 3
---

Нечитаемые данные обрабатываются по-разному при создании структуры и после
открытия поля. Удаление ключа — третий, отдельный случай.
Если значение читается, но требует исправления, используйте
[правило](/amethystate/ru/state/rules/).

## Открытие

По умолчанию нечитаемое значение не даёт открыть структуру. `new_with`
паникует с путём и ошибкой кодека, а `try_new_with` возвращает ошибку.
Эта политика называется `Refuse`.

При `UseDefault` структура создаётся, а поле отображает свой default.
Исходные данные сохраняются для восстановления.
[`try_get`](/amethystate/ru/primitives/field/) сообщает о нечитаемом значении,
пока не придёт читаемое обновление:

<!-- shown: a struct that opens over a value it cannot read -->
```rust
#[amethystate(prefix = "mixed", on_unreadable = UseDefault)]
pub struct Mixed {
    #[amestate(default = 8080u16)]
    pub port: u16,

    #[amestate(default = "".to_string(), on_unreadable = Refuse)]
    pub licence: String,
}
```
<!-- /shown -->

**Поле может ужесточить политику.** В примере нечитаемый `port` заменяется
default, но нечитаемый `licence` не даёт создать структуру. В структуре
с `Refuse` нельзя явно задать полю `UseDefault`: макрос отклонит такое сочетание.
Вложенные структуры так же наследуют и могут ужесточать политику родителя.

**Неуказанные политики берутся из store.** Настройка поля приоритетнее настройки
структуры, а настройка структуры — настройки store. Приложение может задать
общую политику, не заменяя явные решения в типах состояния.
См. [Открытие store](/amethystate/ru/store/opening/).

**Разбирайте вариант ошибки, а не её сообщение.** `try_new_with` возвращает `OpenStruct`:

<!-- shown: telling one failed open from another -->
```rust
match Strict::try_new_with(&store) {
    Ok(_) => {}
    Err(OpenStruct::WillNotRead { at, why }) => eprintln!("{at} is unreadable: {why}"),
    Err(OpenStruct::Taken(taken)) => {
        eprintln!("{} already holds {}", taken.held_by, taken.at)
    }
    Err(other) => return Err(other.into()),
}
```
<!-- /shown -->

При `UseDefault` структура создаётся, но поле предоставляет путь и причину
через `Disagreement`. Он описывает расхождение кэша с хранилищем,
а не новую операцию чтения:

<!-- shown: asking a field what the store disagrees with -->
```rust
let held = match state.port().try_get() {
    Ok(port) => port,
    Err(no) => {
        match no.reason {
            Reason::WillNotRead(said) => eprintln!("{} will not decode: {said}", no.at),
            Reason::Occupied(said) => eprintln!("{} was already taken: {said}", no.at),
            _ => eprintln!("{} is not what the store has", no.at),
        }

        state.port().get()
    }
};
```
<!-- /shown -->

Причина указывает, что произошло:

- `WillNotRead`: кодек не смог декодировать сохранённое значение.
- `Occupied`: поле не смогло записать default, поскольку по пути уже были данные.
- `Reason::Closed`: store закрыт; поле хранит значение в кэше.
- `Reason::NotWrittenBack`: [правило](/amethystate/ru/state/rules/) исправило
  сохранённое значение, но записать исправление обратно не удалось.

`Reason` помечен `#[non_exhaustive]`: в `match` нужна ветка `_`.
У перечислений ошибок операций другая политика — см.
[Ошибки](/amethystate/ru/concepts/errors/).

## Ключ удалили под живым полем

По умолчанию `on_delete = Keep` сохраняет текущее значение поля после удаления
его ключа. Выберите `UseDefault`, чтобы вместо этого сбрасывать поле в default:

<!-- shown: a field that wants the default back when its key goes -->
```rust
#[amethystate(prefix = "mixed_delete")]
pub struct MixedDelete {
    #[amestate(default = 800u32)]
    pub width: u32,

    #[amestate(default = 600u32, on_delete = UseDefault)]
    pub height: u32,
}
```
<!-- /shown -->

## Пришло изменение, и оно не декодируется

Если обновление не декодируется, поле сохраняет последнее читаемое значение
и не отправляет подписчикам некорректное обновление. `try_get` сообщает
о проблеме, пока не придёт читаемое значение. Отдельная политика для этого не нужна.
