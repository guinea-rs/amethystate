---
title: GPUI
---


GPUI использует модель сущностей с отложенным оповещением: изменения происходят внутри замыканий обновления сущности, а фреймворк оповещает зависимых после возврата из замыкания. Синхронные подписки `Field<T>` с этой моделью напрямую не складываются, поэтому интеграция соединяет их через асинхронный канал.

## Как это работает

`amethystate-gpui` даёт `AmeView<T>` — обёртку, которая держит структуру состояния и `ReactiveScope`. При создании она подписывается на все внешние изменения этой структуры и отправляет пустое сообщение в неограниченный канал. Фоновая задача внутри сущности вычерпывает этот канал и вызывает `entity_cx.notify()`, что запускает перерисовку GPUI.

То есть GPUI читает состояние синхронно во время `render` через `.get()`, а изменения обнаруживаются асинхронно в фоне.

## Установка

```toml
[dependencies]
amethystate-gpui = "0.23"
```

Откройте store до того, как открывать окна:

```rust
StoreBuilder::new("./app.redb").init_global();
```

## Объявление состояния

```rust
#[amethystate(prefix = "counter")]
pub struct CounterState {
    #[amestate(default = 0)]
    pub count: i32,
}
```

## Создание сущности

Чтобы обернуть структуру состояния в `AmeEntity`, используйте `cx.new_amethystate()` вместо `cx.new()`:

```rust
struct CounterView {
    state: AmeEntity<CounterState>,
}

impl CounterView {
    fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new_amethystate(CounterState::new);
        Self { state }
    }
}
```

Если структура не открылась, `new_amethystate` паникует и печатает ошибку. `try_new_amethystate` принимает то же замыкание, но ошибку возвращает.

`AmeEntity<T>` - псевдоним для `Entity<AmeView<T>>`. `AmeView` разыменовывается в `T`, поэтому к полям состояния обращаются прямо через сущность.

## Чтение состояния в render

```rust
impl Render for CounterView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_count = self.state.read(cx).count().get();

        div().child(format!("Count: {}", current_count))
    }
}
```

## Запись состояния

Писать можно откуда угодно — из обработчика `on_click` сущности, из фонового потока, из другой части приложения. Сущность подписана на своё состояние через `external`, поэтому оповещает её чужая запись: запись через форк сама запускает `notify()` и перерисовку. Запись через собственный хендл сущности несёт её же id и пропускается, так что обработчик, который её сделал, зовёт `notify()` сам:

```rust
// from a click handler inside render
let state = self.state.clone();
Button::new("Increment")
    .on_click(move |_, _, cx| {
        state.read(cx).count().update(|v| v + 1).ok();
        state.update(cx, |_, cx| cx.notify());
    })

// from a background thread via fork
let forked = state.read(cx).fork();
std::thread::spawn(move || {
    loop {
        std::thread::sleep(Duration::from_secs(2));
        forked.count().update(|v| v + 1).ok();
    }
});
```

У фонового потока нет `cx`, чтобы позвать `notify()`, поэтому он пишет через форк, и такие записи сущность слышит сама.

## Какой GPUI

Адаптер собран на [`gpui-pre`](https://crates.io/crates/gpui-pre) 0.3.6, выпуске GPUI на crates.io, и ему нужен Rust 1.95. Зависьте от того же пакета под именем `gpui`, тогда у приложения и адаптера будет одна копия крейта:

```toml
[dependencies]
gpui = { package = "gpui-pre", version = "0.3.6" }
```

Две копии, скажем адаптерная и ещё одна из git, для Cargo - два разных крейта, и типы у них не совпадут.

## Примеры

- [`gpui`](https://github.com/guinea-rs/amethystate/tree/master/examples/gpui)
