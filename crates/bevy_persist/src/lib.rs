use std::{
    any::{Any, TypeId},
    sync::{Arc, Mutex},
    time::Duration,
};

use bevy_app::{prelude::*, AppExit};
use bevy_ecs::{bevy_utils::HashMap, prelude::*};
use notify::{RecursiveMode, Watcher};

#[cfg(feature = "dynamic")]
pub fn load_game(name: &str) {
    use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};

    let persist_context_inner = Arc::new(Mutex::new(PersistContextInner {
        should_reload: false,
        should_exit: false,
        serde_resources: HashMap::default(),
        raw_resources: HashMap::default(),
    }));

    let lib_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("{}{}{}", DLL_PREFIX, name, DLL_SUFFIX));
    let lib_path2 = lib_path.clone();

    let persist_context_inner2 = persist_context_inner.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::watcher(tx, Duration::new(0, 0)).unwrap();
    watcher
        .watch(&lib_path, RecursiveMode::NonRecursive)
        .unwrap();

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            println!("Reloading...");
            persist_context_inner2.lock().unwrap().should_reload = true;
            watcher
                .watch(&lib_path2, RecursiveMode::NonRecursive)
                .unwrap();
        }
    });

    loop {
        persist_context_inner.lock().unwrap().should_reload = false;
        let game = libloading::Library::new(&lib_path).unwrap();
        unsafe {
            let func: libloading::Symbol<fn(AppBuilder)> = game.get(b"__bevy_the_game").unwrap();
            func(PersistContextInner::new_app(&persist_context_inner));
        }
        if persist_context_inner.lock().unwrap().should_exit {
            return;
        }
    }
}

pub struct PersistContext {
    resources_save: Vec<Box<dyn FnOnce(&mut Resources) + Send + Sync>>,
    inner: Arc<Mutex<PersistContextInner>>,
}

struct PersistContextInner {
    should_reload: bool,
    should_exit: bool,
    serde_resources: HashMap<&'static str, Vec<u8>>,
    raw_resources: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

pub trait RestoreResource {
    // FIXME make it possible to use resources implementing bevy_reflect::Reflect.

    fn add_serde_restore_resource<T>(&mut self, res: T) -> &mut Self
    where
        T: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static;

    fn add_raw_restore_resource<T>(&mut self, res: T) -> &mut Self
    where
        T: Send + Sync + 'static;
}

impl RestoreResource for Resources {
    fn add_serde_restore_resource<T>(&mut self, mut res: T) -> &mut Self
    where
        T: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static,
    {
        if let Some(mut ctx) = self.get_mut::<PersistContext>() {
            ctx.resources_save.push(Box::new(|res| {
                let serialized = bincode::serialize(&*res.get::<T>().unwrap()).unwrap();
                res.get::<PersistContext>()
                    .unwrap()
                    .inner
                    .lock()
                    .unwrap()
                    .serde_resources
                    .insert(std::any::type_name::<T>(), serialized);
            }));

            if let Some(serialized) = ctx
                .inner
                .lock()
                .unwrap()
                .serde_resources
                .get(std::any::type_name::<T>())
            {
                res = bincode::deserialize(serialized).unwrap();
            }
        }
        self.insert(res);
        self
    }

    fn add_raw_restore_resource<T>(&mut self, mut res: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        if let Some(mut ctx) = self.get_mut::<PersistContext>() {
            ctx.resources_save.push(Box::new(|res| {
                let raw = res.take_global_only_resource::<T>().unwrap();
                res.get::<PersistContext>()
                    .unwrap()
                    .inner
                    .lock()
                    .unwrap()
                    .raw_resources
                    .insert(TypeId::of::<T>(), Box::new(raw));
            }));

            if let Some(stored_res) = ctx
                .inner
                .lock()
                .unwrap()
                .raw_resources
                .remove(&TypeId::of::<T>())
            {
                res = *(stored_res as Box<dyn Any>)
                    .downcast::<T>()
                    .unwrap_or_else(|_| unreachable!("Wrong type"));
            }
        }
        self.insert(res);
        self
    }
}

impl RestoreResource for AppBuilder {
    fn add_serde_restore_resource<T>(&mut self, res: T) -> &mut Self
    where
        T: serde::Serialize + for<'a> serde::Deserialize<'a> + Send + Sync + 'static,
    {
        self.resources_mut().add_serde_restore_resource(res);
        self
    }

    fn add_raw_restore_resource<T>(&mut self, res: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.resources_mut().add_raw_restore_resource(res);
        self
    }
}

impl PersistContextInner {
    fn new_app(this: &Arc<Mutex<Self>>) -> AppBuilder {
        let mut app = App::build();
        app.add_system_to_stage(stage::LAST, probe_for_exit.system());
        app.add_system_to_stage(stage::LAST, probe_for_reload.system());
        app.add_resource(PersistContext {
            inner: this.clone(),
            resources_save: vec![],
        });
        app.add_event::<AppReload>();
        app
    }
}

fn probe_for_reload(_: &mut World, res: &mut Resources) {
    let should_reload = res
        .get::<PersistContext>()
        .unwrap()
        .inner
        .lock()
        .unwrap()
        .should_reload;
    if should_reload {
        // TODO: Maybe persist everything for which `register_component` or `register_properties`
        // was called?
        let resources_save =
            std::mem::take(&mut res.get_mut::<PersistContext>().unwrap().resources_save);
        for resource_save in resources_save {
            resource_save(&mut *res);
        }
        res.get_mut::<Events<AppReload>>().unwrap().send(AppReload);
    }
}

fn probe_for_exit(_: &mut World, res: &mut Resources) {
    if let Some(app_exit_events) = res.get::<Events<AppExit>>() {
        if app_exit_events
            .get_reader()
            .earliest(&app_exit_events)
            .is_some()
        {
            res.get::<PersistContext>()
                .unwrap()
                .inner
                .lock()
                .unwrap()
                .should_exit = true;
        }
    }
}

/// An event that indicates the app should reload.
pub struct AppReload;
