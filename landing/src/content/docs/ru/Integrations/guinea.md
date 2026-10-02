---
title: guinea
---

`amethystate-guinea` даёт таймеру [guinea](https://crates.io/crates/guinea)
период, который следует за значением в store. Настройку поменяли — из
интерфейса, из другого потока или правкой файла, — и таймер сразу
перезаводится с новым периодом, не дожидаясь конца старого.

## Установка

```toml
[dependencies]
amethystate-guinea = "0.23"
```

Версия у крейта та же, что у amethystate. Собирается он с guinea 0.18 и, как и guinea, требует Rust 1.95.

## Период по полю

```rust
use amethystate_guinea::IntoChanging;
use guinea::timers::Period;
use std::time::Duration;

cx.every(
    Period::follows(settings.ping_interval_ms().changing(), Duration::from_millis),
    &agent,
    || Ping,
);
```

`changing()` превращает поле в то, чего ждёт `Period::follows`. guinea читает
значение перед каждым ожиданием и следит за полем, пока таймер работает:
запись обрывает текущее ожидание, и следующий тик приходит через новый период
после неё. Слежка кончается вместе с таймером или когда `Timer::period` даёт
ему другой период.

Записать можно из любого потока. Перезаводит таймер guinea сам, на потоке
интерфейса.

## Период по записи карты

`changing()` есть и у `ReactiveCell`, через него и следят за записью карты:

```rust
let ping = intervals.entry_cell("ping".to_string());

cx.every(Period::follows(ping.changing(), Duration::from_millis), &agent, || Ping);
```

Пока ключа нет, читать ячейке нечего, и таймер не тикает. Он пойдёт, как только
запись вставят, и встанет снова, если её удалят или карту дропнут.
