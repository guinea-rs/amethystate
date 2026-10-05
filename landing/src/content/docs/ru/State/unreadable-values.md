---
title: Когда значение не читается
sidebar:
  order: 3
---

Три момента, у каждого свой ответ: структура открывается над байтами, которые
не декодируются; под живым полем удалили ключ; пришло изменение, и оно не
декодируется. Значение, которое декодируется прекрасно и всё равно бессмысленно,
— другое дело, и отвечают на него [Правила](/amethystate/ru/state/rules/).

## Открытие

Если по объявленному пути лежит то, что не декодируется в тип поля, структура
не построится и назовёт этот путь. Это `Refuse`, он по умолчанию: `new_with`
паникует, называя путь и то, что сказал кодек, а `try_new_with` отдаёт то же
самое как `Err`. `UseDefault`
— для приложения, которое обязано запуститься при любом раскладе: поле берёт
значение из `default`, испорченное значение остаётся на диске, чтобы его
кто-нибудь починил, а [`try_get`](/amethystate/ru/primitives/field/) отвечает
`Err` — с самого построения и до первого изменения, которое декодируется.

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

**Поле вправе ужесточить то, что сказала структура.** Выше настройки откроются
со сломанным `port`, а нечитаемый `licence` остановит всё. Обратное — `Refuse`
на структуре и `UseDefault` на поле — не соберётся, и компилятор назовёт это
поле. Вложенная структура наследует ответ той, что её держит, ужесточает его
так же и сверяется с ней на компиляции.

**Где не сказал никто, говорит store.** Сказанное полем бьёт сказанное
структурой, а сказанное структурой — то, с чем store открыли. Так что
приложение один раз говорит, чего хочет от всего, у чего мнения не было, — а
всё объявленное выше остаётся нетронутым:
[Как открывают store](/amethystate/ru/store/opening/).

**Неудавшееся построение говорит, как именно оно не удалось.** `try_new_with`
отдаёт `OpenStruct` — те способы упасть, что есть у конструктора, и никаких
других, — так что один случай отличают от другого по варианту, а не по тексту:

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

При `UseDefault` ничего не падает, и то же место с той же причиной приходят
через поле — как `Disagreement`: это не провал самого вопроса, а то, о чём поле
и store не договорились.

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

Байты, которые не декодировались, приходят как `WillNotRead` — с фразой самого
кодека, — а поле, так и не записавшее свой `default`, потому что store уже
что-то там держал, как `Occupied`. Поле закрытого store отвечает
`Reason::Closed`: значение в нём последнее, что оно слышало. А
`Reason::NotWrittenBack` значит, что [правило](/amethystate/ru/state/rules/)
поправило сохранённое значение, а store поправку не взял.

`Reason` помечен `#[non_exhaustive]` — это список того, в чём поле и store
бывают не согласны, и он растёт, — поэтому `match` по нему держит ветку `_`.
Наборы её не держат, и [Ошибки](/amethystate/ru/concepts/errors/) объясняют
почему.

## Ключ удалили под живым полем

Поле продолжает показывать последнее, что держало, — то самое, что было на
экране мгновение назад. Против него `default` всего лишь догадка, сделанная на
компиляции. `UseDefault` просит именно догадку:

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

Поле держит последнее значение, с которым store был согласен, подписчиков
никто не зовёт. `try_get` про это скажет и замолчит, как только очередное
изменение декодируется. Объявлять тут нечего.
