---
title: TypeScript
---

`amethystate` даёт npm-пакет для приложений Tauri, у которых фронтенд на TypeScript или JavaScript. На нём стоят сгенерированные биндинги: по классу на каждую структуру состояния, в классе `Field<T>` на каждое значение и `ReactiveMap<V>` на каждую карту.

## Установка

```sh
npm install amethystate
```

`@tauri-apps/api` - peer-зависимость, и она уже должна быть в вашем проекте.

## Кодогенерация

Классы получаются из объявлений на Rust. Типы значений, которые в них лежат, даёт [`ts-rs`](https://github.com/Aleph-Alpha/ts-rs): выведите `TS` для тех типов из полей и карт, что не примитивы. До фронтенда доходят только они, значит, и `TS` нужен только им.

**1. Добавьте бинарную цель и зависимости в крейт Tauri:**

```toml
# src-tauri/Cargo.toml
[[bin]]
name = "codegen"
path = "bin/codegen.rs"

[dependencies]
amethystate-codegen = "0.23"
ts-rs = "12"
```

**2. Выведите `TS` для типов значений:**

```rust
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, TS)]
pub struct Todo {
    pub title: String,
    pub done: bool,
}
```

**3. Создайте `bin/codegen.rs`:**

```rust
use your_crate::Todo;
use ts_rs::{Config, TS};

fn main() {
    let beside_the_bindings = Config::new().with_out_dir("../src/bindings");
    Todo::export_all(&beside_the_bindings).expect("Todo would not export");

    amethystate_codegen::amethystate_codegen!(ts_out = "../src/bindings/amethystate.ts");
}
```

Каждый тип значения биндинги берут из файла с его именем рядом с собой. Туда его и пишет `ts-rs`, если дать ему тот же каталог.

**4. Запустите:**

```sh
cargo run --bin codegen
```

## Значения по умолчанию лежат в биндингах

Генератор вписывает в биндинги значение по умолчанию каждого поля — тем JSON, каким его хранит store. Поле, которого в store нет — структуру на стороне Rust ещё ни разу не строили или ключ с тех пор удалили, — загружается с этим значением, а карта, которой в store нет, загружается пустой.

## Загрузка

```ts
import { Todos } from "./bindings/amethystate";

const todos = await Todos.load();
```

`load()` читает всё под префиксом структуры одним вызовом IPC. Вложенная структура становится отдельным классом, до которого добираются через свойство. `dispose()` у структуры снимает все подписки, которые взяли его поля и карты.

## Поля

```ts
todos.hideDone.get();
await todos.hideDone.set(true);
await todos.nextId.update((id) => id + 1);
```

Запись принимается сразу: `get()` и все подписчики видят новое значение раньше, чем ответит store. Если store запись отверг, она откатывается, подписчики снова получают старое значение, а промис отклоняется.

## Подписки

```ts
const stop = todos.hideDone.subscribe((hide) => render(hide));

stop();
```

Подписчика сразу зовут с текущим значением и потом после каждого изменения. `get()` отдаёт одно и то же значение, пока ничего не изменилось, а `subscribe` возвращает функцию, которая подписку снимает. Именно этого требуют `useSyncExternalStore` из React и контракт store в Svelte.

## Карты

Ключи у карты строковые, и в ключе может быть любой символ: точка в ключе остаётся частью одного уровня, как её и пишет store.

```ts
todos.items.get("3");
todos.items.has("3");
todos.items.entries();

await todos.items.insert("3", { title: "milk", done: false });
await todos.items.update("3", { title: "milk", done: true });
await todos.items.remove("3");
await todos.items.clear();

const stopAll = todos.items.subscribe((entries) => render(entries));
const stopOne = todos.items.subscribeKey("3", (todo) => renderRow(todo));
const stopRaw = todos.items.onChange((change) => log(change));
```

`entries()` отдаёт записи по порядку ключей, и это тот же массив, пока ничего не изменилось. `insert` кладёт значение, был ключ или нет, а `update` ключ, которого в карте нет, не примет. Подписчик ключа получает `undefined`, как только ключ пропал, кто бы его ни удалил. `onChange` отдаёт каждое `MapChange` по мере изменений и ничего не отдаёт в момент подписки.

Запись в карту принимается сразу и откатывается, если её отвергли, так же как запись в поле. Изменение, которое сделала сама карта, второй раз не приходит, когда store о нём объявляет.

## Сброс на диск

Бэкенд копит записи. Чтобы они оказались на диске сейчас, например перед закрытием приложения, вызовите `save()` у структуры:

```ts
await todos.save();
```

## Без Tauri

`load()` принимает store, с которым говорит, аргументом, а по умолчанию берёт `tauri()`. Подставить можно всё, что реализует `Transport`: так собственные тесты пакета и обходятся без приложения Tauri.

## Примеры

- [`tauri-typescript`](https://github.com/guinea-rs/amethystate/tree/master/examples/tauri-typescript) — приложение todo поверх пакета
