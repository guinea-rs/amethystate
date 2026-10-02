pub mod commands;
pub mod scope;

use tauri::{
    Manager, RunEvent, Runtime, WindowEvent,
    plugin::{Builder, TauriPlugin},
};

pub fn init<R: Runtime>(store: amethystate::Store) -> TauriPlugin<R> {
    Builder::new("amethystate")
        .invoke_handler(tauri::generate_handler![
            commands::amethystate_get,
            commands::amethystate_set,
            commands::amethystate_subscribe,
            commands::amethystate_unsubscribe,
            commands::amethystate_get_prefix,
            commands::amethystate_flush,
            commands::amethystate_delete,
            commands::amethystate_delete_prefix,
            commands::amethystate_scan_keys,
        ])
        .setup(|app, _api| {
            app.manage(commands::PluginState::new(store));
            Ok(())
        })
        .on_event(|app, event| {
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::Destroyed,
                ..
            } = event
                && let Some(state) = app.try_state::<commands::PluginState>()
            {
                state.forget(label);
            }
        })
        .build()
}
