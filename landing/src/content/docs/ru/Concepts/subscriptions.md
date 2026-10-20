---
title: Подписки
sidebar:
  order: 17
---

У полей, ячеек и карт одинаковая модель подписок. Здесь описаны время жизни
подписки, потоки колбэков и определение источника записи.

## Подписка живёт, пока жив guard

<!-- shown: subscribing, and letting the subscription go -->
```rust
let sub = state.port().subscribe(move |port| {
    seen.lock().unwrap().push(*port);
});

state.port().set(9090);
assert_eq!(*heard.lock().unwrap(), [9090]);

drop(sub);

state.port().set(1234);
assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

`subscribe` возвращает guard, который владеет подпиской. Храните его, пока нужны
уведомления: удаление guard отменяет регистрацию колбэка. Отдельного вызова
`unsubscribe` нет.

## Держать guard — забота вызывающего

Не присваивайте guard в `_`: он удалится в конце выражения, и последующие
изменения уже не вызовут колбэк. Именованная переменная вроде `_sub` сохраняет
guard до конца области видимости:

<!-- shown: a subscription nobody held on to -->
```rust
let ignored = Arc::clone(&heard);
let _ = state.port().subscribe(move |port| {
    ignored.lock().unwrap().push(*port);
});

state.port().set(4321);
assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

В `heard` осталось только `[9090]` от первой подписки. Храните guard рядом
с компонентом или состоянием, которому нужны уведомления, чтобы время их жизни
совпадало.

### Несколько сразу

<!-- shown: keeping several subscriptions in one place -->
```rust
let mut scope = ReactiveScope::new();

state
    .port()
    .subscribe(|port| println!("port {port}"))
    .watch(&mut scope);
state
    .host()
    .subscribe(|host| println!("host {host}"))
    .watch(&mut scope);

scope.clear();
```
<!-- /shown -->

`ReactiveScope` владеет несколькими guard. `clear` или удаление scope
отменяет все его подписки.

## Когда обычной подписки недостаточно

Для дополнительных настроек используйте `subscription_with()`:

| метод | поведение |
| --- | --- |
| `.external()` | Отфильтровать собственные обновления значений этого хендла. |
| `.key(k)` | Следить за одной записью карты. |
| `.register(f)` | Зарегистрировать колбэк и вернуть guard. |
| `.register_with_source(f)` | Передать в колбэк ID записавшего хендла. |
| `.stream()` | Получать изменения как `Stream`. |

## Как забрать изменения в собственный цикл

Колбэки требуют `Send + Sync`: наблюдатель файлов может вызвать их в фоновом
потоке. Это исключает состояние на `Rc` и многие хендлы UI.

`.stream()` вместо этого передаёт значения в поток событий. Обрабатывайте его
в потоке, которому принадлежит состояние интерфейса:

<!-- shown: taking the changes into a loop of your own -->
```rust
let mut ports = state.port().subscription_with().stream();

state.port().set(9090);
state.port().set(1234);

let mut heard = Vec::new();
futures::executor::block_on(async {
    while let Some(port) = ports.next().await {
        heard.push(port);
        if port == 1234 {
            break;
        }
    }
});

assert_eq!(heard, [9090, 1234]);
```
<!-- /shown -->

Поток сохраняет отдельные изменения, не объединяя их. При необходимости
объединяйте события в своём обработчике. Удаление stream отменяет подписку.

## Чья это была запись

Запись содержит ID хендла-писателя. `.external()` использует его для
фильтрации записей подписавшегося хендла:

<!-- shown: hearing only about somebody else's writes -->
```rust
let watcher = state.port().fork();

let _sub = state
    .port()
    .subscription_with()
    .external()
    .register(move |port| {
        seen.lock().unwrap().push(*port);
    });

state.port().set(8080);
watcher.set(9090);

assert_eq!(*heard.lock().unwrap(), [9090]);
```
<!-- /shown -->

Например, передайте рабочему потоку fork, а UI подпишите с `.external()`.
Интерфейс получит изменения рабочего потока без повторных уведомлений
о собственных записях значений.

У внешней правки файла ID нет, поэтому external-подписки тоже её получают.

### `clone` и `fork`

Оба хендла обращаются к одному значению, но по-разному определяют писателя:

<!-- shown: the same actor, and a different one -->
```rust
let port = state.port();
let same = port.clone();
let other = port.fork();

let _sub = port
    .subscription_with()
    .external()
    .register(move |value| seen.lock().unwrap().push(*value));

same.set(1111);
other.set(2222);

assert_eq!(*heard.lock().unwrap(), [2222]);

assert_eq!(port.instance_id(), same.instance_id());
assert_ne!(port.instance_id(), other.instance_id());
```
<!-- /shown -->

`clone` сохраняет ID: оригинал и клон считаются одним писателем для
`.external()`. `fork` получает новый ID, поэтому хендлы видят записи друг друга.

`instance_id()` поля или карты возвращает этот ID для сравнения или лога.

### Автор приходит вместе со значением

<!-- shown: asking who made the change -->
```rust
let _sub = state
    .port()
    .subscription_with()
    .register_with_source(move |port, who| {
        seen.lock().unwrap().push((*port, who));
    });

state.port().set(9090);

let (port, who) = heard.lock().unwrap()[0];
assert_eq!(port, 9090);
assert_eq!(who, Some(state.port().instance_id()));
```
<!-- /shown -->

`register_with_source` передаёт ID вместе со значением. Он нужен,
если колбэк решает, как обработать каждое событие отдельно.

## `external` на `ReactiveMap`

На карте `.external()` фильтрует только `Update`. Структурные изменения —
`Insert`, `Remove` и `Clear` — приходят всем подписчикам. Влияние на `insert`
разобрано в [ReactiveMap](/amethystate/ru/primitives/reactive-map/#что-фильтрует-external).
