---
title: Объявление шагов миграции
sidebar:
  order: 21
---


## Макрос #[migrate]

`#[migrate]` превращает обычную функцию в зарегистрированный шаг миграции. Всё, что макросу нужно, он выводит из сигнатуры функции:

- исходную версию - из типа аргумента (`AmeData<v1::Config>`)
- целевую версию - из возвращаемого типа (`AmeData<Config>`)
- описание шага - из имени функции

```rust
#[migrate]
fn migrate_config_v1_to_v2(old: AmeData<v1::Config>) -> amethystate::MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        address: old.host,
        port: old.port,
    })
}
```

Вызовите `StoreBuilder::migrate`, чтобы собрать и выполнить функции
с `#[migrate]`, включённые в бинарник. Вести отдельный список регистраций не нужно.

## Версионирование старых структур

Миграции нужна старая схема в виде типа, поэтому её объявление нужно оставить
в коде. Путь модуля не влияет на хранение. В примерах старые объявления лежат
в `mod v1`, чтобы текущая версия могла использовать то же имя структуры.

```rust
mod v1 {
    use super::*;

    #[amethystate(prefix = "app", version = 1)]
    pub struct Config {
        #[amestate(default = "localhost".to_string())]
        pub host: String,

        #[amestate(default = 8080)]
        pub port: u16,
    }
}

#[amethystate(prefix = "app", version = 2)]
pub struct Config {
    #[amestate(default = "localhost".to_string())]
    pub address: String,

    #[amestate(default = 8080)]
    pub port: u16,
}
```

У обеих структур один и тот же `prefix`. Номер версии - это то, по чему мигратор определяет, какой шаг выполнять.

## AmeData

`AmeData<T>` - простой контейнер данных, повторяющий поля структуры `#[amethystate]` без реактивных обёрток. Именно из него читают и в него пишут внутри шага миграции.

К полям обращаются напрямую:

```rust
fn migrate_config_v1_to_v2(old: AmeData<v1::Config>) -> amethystate::MigrationResult<AmeData<Config>> {
    Ok(AmeData::<Config> {
        address: old.host, // direct field access
        port: old.port,
    })
}
```

## Объявление переименований

`#[rename(old_field => new_field)]` объявляет, что поле было переименовано между версиями. Их можно ставить несколько, для нескольких переименований. Каждый `#[rename]` порождает проверку на этапе компиляции, что оба поля есть на соответствующих типах, — опечатка становится ошибкой компиляции:

```rust
#[migrate]
#[rename(login => username, tier => plan)]
fn migrate_identity_v1_to_v2(
    old: AmeData<v1::Identity>,
) -> amethystate::MigrationResult<AmeData<Identity>> {
    Ok(AmeData::<Identity> {
        username: old.login,
        plan: match old.tier.as_str() {
            "pro" => "professional".to_string(),
            other => other.to_string(),
        },
        created_at_ms: 1_700_000_000_000,
    })
}
```

`#[rename]` - это объявление, а не реализация. Само сопоставление полей всё равно пишут руками в теле функции. Атрибут существует, чтобы породить проверку на этапе компиляции и записать переименование в историю схемы.

## Что шаг вычищает сам

После сгенерированного шага библиотека удаляет пути, которыми владела старая
схема и не владеет новая. При удалении или переименовании карты удаляются
и её старые записи: карта владеет всем своим поддеревом. Удалять записи
по одной не нужно. См. [Кто владеет каким местом](/amethystate/ru/concepts/claims/).

Если карта есть в обеих схемах, её записи обрабатываются иначе. Отсутствие
`obsolete` в возвращённой карте `routes` само по себе не удалит
`routes.obsolete` из хранилища: новые записи записываются по одной.
Примите `MigrationContext` вторым аргументом и удалите ненужный ключ явно:

```rust
#[migrate]
fn migrate_proxy_config_v1_to_v2(
    old: AmeData<v1::ProxyConfig>,
    ctx: &mut MigrationContext,
) -> amethystate::MigrationResult<AmeData<ProxyConfig>> {
    ctx.delete("routes.obsolete")?;

    let routes = old
        .routes
        .into_iter()
        .filter(|(key, _)| key != "obsolete")
        .collect();

    Ok(AmeData::<ProxyConfig> {
        name: old.name,
        routes,
    })
}
```

`ctx` ограничен префиксом линии: `ctx.delete("routes.api")` удаляет `network.routes.api`, а `ctx.delete_prefix("routes")` одним вызовом снимает уровень со всем, что под ним. Полное API контекста: [Ручные миграции](/amethystate/ru/migrations/manual/).

## Пути в несколько шагов

Если префикс проходит больше одной версии, объявите шаг на каждый переход. Мигратор выстраивает их по порядку:

```rust
// v1 → v2
#[migrate]
#[rename(title => name)]
fn migrate_workspace_v1_to_v2(
    old: AmeData<workspace_v1::Workspace>,
) -> amethystate::MigrationResult<AmeData<workspace_v2::Workspace>> {
    Ok(AmeData::<workspace_v2::Workspace> {
        name: old.title,
        appearance_theme: old.theme,
    })
}

// v2 → v3
#[migrate]
fn migrate_workspace_v2_to_v3(
    old: AmeData<workspace_v2::Workspace>,
) -> amethystate::MigrationResult<AmeData<Workspace>> {
    Ok(AmeData::<Workspace> {
        name: old.name,
        appearance_theme: old.appearance_theme,
        welcome_title: "Welcome".to_string(),
    })
}
```

Каждому шагу нужно знать только про версию непосредственно перед ним.
