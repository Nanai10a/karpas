use std::collections::HashMap;

use bevy::app::{App, Plugin, PluginGroup, PluginGroupBuilder};
use bevy::asset::HandleUntyped;
use bevy::DefaultPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(KarpasPlugins)
        .run();
}

struct KarpasPlugins;
impl PluginGroup for KarpasPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(ConfigPlugin);
        group.add(StagePlugin);
        group.add(AssetPlugin);
        group.add(LogPlugin);

        group.add(stag::initial::Plugin);
        group.add(stag::title::Plugin);
        group.add(stag::game::Plugin);
        group.add(stag::end::Plugin);
    }
}

struct ConfigPlugin;
impl Plugin for ConfigPlugin {
    fn name(&self) -> &str { "config" }

    fn build(&self, app: &mut App) {
        let config = Config {
            path: PathConfig {
                font: "fonts/zkgn/ZenKakuGothicNew-Regular.ttf",
            },
        };

        app.insert_resource(config);
    }
}

struct Config {
    path: PathConfig,
}

struct PathConfig {
    font: &'static str,
}

struct StagePlugin;
impl Plugin for StagePlugin {
    fn name(&self) -> &str { "stage" }

    fn build(&self, app: &mut App) { app.add_state(Stage::Initial).add_system(end_on_3_esc_press); }
}

use bevy::core::{Stopwatch, Time};
use bevy::ecs::system::ResMut;
use bevy::input::keyboard::KeyCode;
use bevy::input::Input;

fn end_on_3_esc_press(
    mut count: Local<u8>,
    time: Res<Time>,
    mut stopwatch: Local<Stopwatch>,
    key: Res<Input<KeyCode>>,
    mut stage: ResMut<State<Stage>>,
) {
    const THRESHOLD: u128 = 150;

    stopwatch.tick(time.delta());

    if key.just_pressed(KeyCode::Escape) {
        stopwatch.unpause();

        if stopwatch.elapsed().as_millis() < THRESHOLD {
            *count += 1;
        } else {
            *count = 0;
        }

        stopwatch.reset();

        if *count >= 3 {
            stage.replace(Stage::End).unwrap();
        }
    }

    if stopwatch.elapsed().as_secs() >= 1 {
        stopwatch.pause();
        stopwatch.reset();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Stage {
    Initial,
    Title,
    Game,
    End,
}

struct AssetPlugin;
impl Plugin for AssetPlugin {
    fn name(&self) -> &str { "asset" }

    fn build(&self, app: &mut App) {
        app.insert_resource(AssetStore {
            store: HashMap::new(),
        });
    }
}

struct AssetStore {
    store: HashMap<&'static str, HandleUntyped>,
}

use bevy::ecs::schedule::State;
use bevy::ecs::system::{Local, Res};

struct LogPlugin;
impl Plugin for LogPlugin {
    fn name(&self) -> &str { "log" }

    fn build(&self, app: &mut App) { app.add_system(on_stage_changed); }
}

fn on_stage_changed(mut before: Local<Option<Stage>>, stage: Res<State<Stage>>) {
    if before.is_none() {
        *before = Some(*stage.current());

        bevy::log::info!("initial stage \"{:?}\"", *stage.current());
        return;
    }

    if before.unwrap() != *stage.current() {
        bevy::log::info!(
            "changed stage \"{:?}\" -> \"{:?}\"",
            before.unwrap(),
            *stage.current()
        );

        *before = Some(*stage.current());
    }
}

mod stag {
    pub mod initial {
        use bevy::app::{App, Plugin as PluginTrait};
        use bevy::asset::AssetServer;
        use bevy::ecs::event::{EventReader, EventWriter};
        use bevy::ecs::schedule::{State, SystemSet};
        use bevy::ecs::system::{Res, ResMut};

        use crate::Stage::Initial as SelfStage;
        use crate::{AssetStore, Config, Stage};

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "initial" }

            fn build(&self, app: &mut App) {
                app.add_event::<Loaded>();
                app.add_system_set(SystemSet::on_enter(SelfStage).with_system(load_assets));
                app.add_system_set(SystemSet::on_update(SelfStage).with_system(detect_loaded));
            }
        }

        struct Loaded;

        fn load_assets(
            asset_server: Res<AssetServer>,
            config: Res<Config>,
            mut store: ResMut<AssetStore>,
            mut loaded: EventWriter<Loaded>,
        ) {
            store
                .store
                .insert("font-zen", asset_server.load_untyped(config.path.font));

            loaded.send(Loaded);
        }

        fn detect_loaded(loaded: EventReader<Loaded>, mut stage: ResMut<State<Stage>>) {
            if !loaded.is_empty() {
                stage.set(Stage::Title).unwrap();
            }
        }
    }

    pub mod title {
        use bevy::app::{App, Plugin as PluginTrait};
        use bevy::ecs::event::{EventReader, EventWriter};
        use bevy::ecs::schedule::{State, SystemSet};
        use bevy::ecs::system::{Commands, Res, ResMut};
        use bevy::input::keyboard::KeyCode;
        use bevy::input::Input;

        use crate::Stage::Title as SelfStage;

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "title" }

            fn build(&self, app: &mut App) {
                app.add_event::<CursorInput>();
                app.add_state(CursorState::Start);
                app.add_system_set(SystemSet::on_enter(SelfStage).with_system(spawn_ui));
                app.add_system_set(SystemSet::on_update(SelfStage).with_system(cursor_input));
                app.add_system_set(SystemSet::on_exit(SelfStage).with_system(despawn_ui));
            }
        }

        enum CursorInput {
            Up,
            Down,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        enum CursorState {
            Start,
            Settings,
            Infos,
            Exit,
        }

        use bevy::render::color::Color;
        use bevy::text::{HorizontalAlign, Text, TextAlignment, TextStyle, VerticalAlign};
        use bevy::ui::entity::TextBundle;

        use crate::AssetStore;

        fn spawn_ui(mut commands: Commands, assets: Res<AssetStore>) {
            let font = assets
                .store
                .get("font-zen")
                .as_ref()
                .unwrap()
                .clone_weak()
                .typed();

            commands.spawn().insert(UiEntity).insert_bundle(TextBundle {
                text: Text::with_section(
                    "",
                    TextStyle {
                        font,
                        font_size: 16.0,
                        color: Color::SALMON,
                    },
                    TextAlignment {
                        vertical: VerticalAlign::Center,
                        horizontal: HorizontalAlign::Center,
                    },
                ),
                ..Default::default()
            });
        }

        use bevy::ecs::component::Component;

        #[derive(Component)]
        struct UiEntity;

        use crate::Config;

        fn cursor_input(
            key: Res<Input<KeyCode>>,
            input: EventWriter<CursorInput>,
            config: Res<Config>,
        ) {
        }

        fn cursor_handle(input: EventReader<CursorInput>, mut state: ResMut<State<CursorState>>) {}

        use bevy::ecs::entity::Entity;
        use bevy::ecs::system::Query;

        fn despawn_ui(mut commands: Commands, entities: Query<(Entity, &UiEntity)>) {
            for (entity, _) in entities.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    pub mod game {
        use bevy::app::{App, Plugin as PluginTrait};
        use bevy::ecs::schedule::SystemSet;

        use crate::Stage::Game as SelfStage;

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "game" }

            fn build(&self, app: &mut App) { app.add_system_set(SystemSet::on_update(SelfStage)); }
        }
    }

    pub mod end {
        use bevy::app::{App, AppExit, Plugin as PluginTrait};
        use bevy::ecs::event::EventWriter;
        use bevy::ecs::schedule::SystemSet;

        use crate::Stage::End as SelfStage;

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "end" }

            fn build(&self, app: &mut App) {
                app.add_system_set(SystemSet::on_enter(SelfStage).with_system(stop_app));
            }
        }

        fn stop_app(mut exit: EventWriter<AppExit>) { exit.send(AppExit); }
    }
}
