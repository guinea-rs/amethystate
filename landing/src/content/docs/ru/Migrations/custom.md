---
title: Переход со своего решения
sidebar:
  order: 24
---

Если существующий TOML-, JSON- или RON-файл хранит поля структуры на верхнем
уровне, его можно попробовать использовать напрямую. Для другой раскладки
нужен явный импорт.

## Файл — это ваша структура

Начните со структуры, которую сейчас загружаете и сохраняете через serde:

<!-- shown: the struct serde was writing -->
```rust
#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub window: Window,
}
```
<!-- /shown -->

Объявите соответствующие поля в корне store:

<!-- shown: the same fields at the top of the store -->
```rust
#[amethystate(mode = "persistent", as_root)]
pub struct Settings {
    #[amestate(default = "light".to_string())]
    pub theme: String,

    #[amestate(default = Window { width: 800, height: 600 })]
    pub window: Window,
}
```
<!-- /shown -->

Включите движок нужного формата:

```toml
amethystate = { version = "0.28", features = ["toml"] }
```

Откройте существующий файл:

<!-- shown: opening the file that was there -->
```rust
let store = StoreBuilder::new(file).backend(Backend::Toml).build()?;
let settings = Settings::load_with(&store);
```
<!-- /shown -->

`as_root` сохраняет расположение полей на верхнем уровне. `load_with` читает
существующие значения и подставляет defaults для отсутствующих. Store создаёт
рядом файл `.meta` — см. [Файлы](/amethystate/ru/store/files/).

При совместимой раскладке TOML и JSON старая структура продолжит читать
сохранённый файл. RON нужно проверить отдельно: движок пишет карту
`{ "theme": .. }`, тогда как выведенная структура RON обычно ожидает `(theme: ..)`.

Если старое объявление переименовывало поля через serde, перенесите эти настройки
в атрибуты состояния — см. [Serde](/amethystate/ru/state/serde/).

## Всё остальное

Для собственной базы, другого формата или раскладки импортируйте значения
кодом приложения. Откройте новый store, прочитайте прежний источник его текущим
ридером, сохраните значения через новую структуру и только затем удалите источник:

<!-- shown: moving the values in once -->
```rust
let mut settings = Settings::load_with(&store);

if let Some(old) = read_old_settings(&old_file)? {
    settings.mutate(|now| {
        now.theme = old.theme;
        now.window = old.window;
    })?;
    std::fs::remove_file(&old_file)?;
}
```
<!-- /shown -->

`mutate` ждёт сохранения. Удаляйте старый файл только после успешного результата.
При остановке до удаления останутся оба источника, и следующий запуск повторит
импорт. Поэтому преобразование должно допускать повторное выполнение.

Первичный импорт не является шагом миграции store: новый пустой store
пропускает исторические шаги. См.
[Чистая установка](/amethystate/ru/migrations/overview/#что-происходит-при-чистой-установке).
