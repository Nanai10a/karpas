use bevy::app::PluginGroupBuilder;
use bevy::core::Stopwatch;
use bevy::prelude::*;
use bevy::utils::HashMap;

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
            key: KeyConfig {
                title: TitleKeyConfig {
                    up: KeyCode::K,
                    down: KeyCode::J,
                    submit: KeyCode::Return,
                },
                game: GameKeyConfig,
            },
        };

        app.insert_resource(config);
    }
}

struct Config {
    path: PathConfig,
    key: KeyConfig,
}

struct PathConfig {
    font: &'static str,
}

struct KeyConfig {
    title: TitleKeyConfig,
    game: GameKeyConfig,
}

struct TitleKeyConfig {
    up: KeyCode,
    down: KeyCode,
    submit: KeyCode,
}

struct GameKeyConfig;

struct StagePlugin;
impl Plugin for StagePlugin {
    fn name(&self) -> &str { "stage" }

    fn build(&self, app: &mut App) { app.add_state(Stage::Initial).add_system(end_on_3_esc_press); }
}

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
    Settings,
    Infos,
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
        use bevy::app::Plugin as PluginTrait;
        use bevy::prelude::*;

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
        use bevy::app::Plugin as PluginTrait;
        use bevy::prelude::*;

        use crate::Stage::Title as SelfStage;
        use crate::{AssetStore, Config, Stage};

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "title" }

            fn build(&self, app: &mut App) {
                app.add_event::<CursorInput>();
                app.add_event::<CursorSubmit>();
                app.insert_resource(CursorState::Start);

                app.add_system_set(SystemSet::on_enter(SelfStage).with_system(spawn_ui));
                app.add_system_set(
                    SystemSet::on_update(SelfStage)
                        .with_system(cursor_input)
                        .with_system(cursor_handle)
                        .with_system(detect_move)
                        .with_system(update_ui),
                );
                app.add_system_set(SystemSet::on_exit(SelfStage).with_system(despawn_ui));
            }
        }

        enum CursorInput {
            Up,
            Down,
            Submit,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        enum CursorState {
            Start,
            Settings,
            Infos,
            Exit,
        }
        impl CursorState {
            fn next(&self) -> Self {
                match *self {
                    Self::Start => Self::Settings,
                    Self::Settings => Self::Infos,
                    Self::Infos => Self::Exit,
                    Self::Exit => Self::Exit,
                }
            }

            fn prev(&self) -> Self {
                match *self {
                    Self::Start => Self::Start,
                    Self::Settings => Self::Start,
                    Self::Infos => Self::Settings,
                    Self::Exit => Self::Infos,
                }
            }

            fn as_str(&self) -> &str {
                match *self {
                    Self::Start => "Start",
                    Self::Settings => "Settings",
                    Self::Infos => "Infos",
                    Self::Exit => "Exit",
                }
            }
        }

        enum CursorSubmit {
            Start,
            Settings,
            Infos,
            Exit,
        }
        impl From<CursorState> for CursorSubmit {
            fn from(from: CursorState) -> Self {
                match from {
                    CursorState::Start => Self::Start,
                    CursorState::Settings => Self::Settings,
                    CursorState::Infos => Self::Infos,
                    CursorState::Exit => Self::Exit,
                }
            }
        }

        fn spawn_ui(mut commands: Commands, assets: Res<AssetStore>) {
            commands
                .spawn()
                .insert(UiEntity)
                .insert_bundle(OrthographicCameraBundle::new_2d())
                .insert_bundle(UiCameraBundle::default());

            let font = assets
                .store
                .get("font-zen")
                .as_ref()
                .unwrap()
                .clone_weak()
                .typed();

            commands
                .spawn()
                .insert(UiEntity)
                .insert_bundle(NodeBundle {
                    style: Style {
                        size: Size {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                        },
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    color: UiColor(Color::NONE),
                    ..default()
                })
                .with_children(|cb| {
                    for text in ["Start", "Settings", "Infos", "Exit"] {
                        cb.spawn().insert(UiEntity).insert_bundle(TextBundle {
                            text: Text::with_section(
                                text,
                                TextStyle {
                                    font: font.clone_weak(),
                                    font_size: 64.0,
                                    color: Color::NONE,
                                },
                                TextAlignment {
                                    vertical: VerticalAlign::Center,
                                    horizontal: HorizontalAlign::Center,
                                },
                            ),
                            ..default()
                        });
                    }
                });
        }

        #[derive(Component)]
        struct UiEntity;

        fn cursor_input(
            key: Res<Input<KeyCode>>,
            mut inputs: EventWriter<CursorInput>,
            config: Res<Config>,
        ) {
            let config = &config.key.title;

            if key.just_pressed(config.up) {
                inputs.send(CursorInput::Up);
            } else if key.just_pressed(config.down) {
                inputs.send(CursorInput::Down);
            } else if key.just_pressed(config.submit) {
                inputs.send(CursorInput::Submit);
            }
        }

        fn cursor_handle(
            mut inputs: EventReader<CursorInput>,
            mut state: ResMut<CursorState>,
            mut moves: EventWriter<CursorSubmit>,
        ) {
            if let Some(input) = inputs.iter().next() {
                match *input {
                    CursorInput::Up => {
                        *state = state.prev();
                    },
                    CursorInput::Down => {
                        *state = state.next();
                    },
                    CursorInput::Submit => {
                        moves.send((*state).into());
                    },
                }
            }
        }

        fn detect_move(mut moves: EventReader<CursorSubmit>, mut stage: ResMut<State<Stage>>) {
            match moves.iter().next() {
                Some(CursorSubmit::Start) => stage.set(Stage::Game).unwrap(),
                Some(CursorSubmit::Settings) => stage.push(Stage::Settings).unwrap(),
                Some(CursorSubmit::Infos) => stage.push(Stage::Infos).unwrap(),
                Some(CursorSubmit::Exit) => stage.set(Stage::End).unwrap(),
                None => (),
            }
        }

        fn update_ui(state: Res<CursorState>, mut entities: Query<(&UiEntity, &mut Text)>) {
            if !state.is_changed() {
                return;
            }

            let state = state.as_str();
            for (_, mut text) in entities.iter_mut() {
                for section in text.sections.iter_mut() {
                    if section.value == state {
                        section.style.color = Color::SALMON;
                    } else {
                        section.style.color = Color::DARK_GRAY;
                    }
                }
            }
        }

        fn despawn_ui(mut commands: Commands, entities: Query<(Entity, &UiEntity)>) {
            for (entity, _) in entities.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    pub mod game {
        use bevy::app::Plugin as PluginTrait;
        use bevy::prelude::*;

        use crate::Stage::Game as SelfStage;

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "game" }

            fn build(&self, app: &mut App) { app.add_system_set(SystemSet::on_update(SelfStage)); }
        }
    }

    pub mod end {
        use bevy::app::{AppExit, Plugin as PluginTrait};
        use bevy::prelude::*;

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
