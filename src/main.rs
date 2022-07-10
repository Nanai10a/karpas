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
                game: GameKeyConfig {
                    left: KeyCode::H,
                    right: KeyCode::L,
                    hard_drop: KeyCode::J,
                    p90_spin: KeyCode::G,
                    n90_spin: KeyCode::S,
                },
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

struct GameKeyConfig {
    left: KeyCode,
    right: KeyCode,
    hard_drop: KeyCode,
    p90_spin: KeyCode,
    n90_spin: KeyCode,
}

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
        use bevy::core::Stopwatch;
        use bevy::prelude::*;

        use crate::AssetStore;
        use crate::Stage::Game as SelfStage;

        pub struct Plugin;
        impl PluginTrait for Plugin {
            fn name(&self) -> &str { "game" }

            fn build(&self, app: &mut App) {
                app.add_event::<FallingInput>();
                app.add_event::<Landing>();

                app.add_system_set(
                    SystemSet::on_enter(SelfStage)
                        .with_system(spawn_ui)
                        .with_system(spawn_area),
                );
                app.add_system_set(
                    SystemSet::on_update(SelfStage)
                        .with_system(update_ui)
                        .with_system(tick_falling)
                        .with_system(falling_input)
                        .with_system(falling_handle)
                        .with_system(handle_landing),
                );
                app.add_system_set(
                    SystemSet::on_exit(SelfStage)
                        .with_system(despawn_ui)
                        .with_system(despawn_area),
                );
            }
        }

        fn spawn_ui(mut commands: Commands, assets: Res<AssetStore>) {
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
                .insert_bundle(UiCameraBundle::default());

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
                        justify_content: JustifyContent::FlexStart,
                        ..default()
                    },
                    color: UiColor(Color::NONE),
                    ..default()
                })
                .with_children(|cb| {
                    cb.spawn()
                        .insert(UiEntity)
                        .insert(ScoreEntity)
                        .insert(Score(0))
                        .insert_bundle(TextBundle {
                            text: Text::with_section(
                                "",
                                TextStyle {
                                    font,
                                    font_size: 48.0,
                                    color: Color::ANTIQUE_WHITE,
                                },
                                TextAlignment {
                                    vertical: VerticalAlign::Center,
                                    horizontal: HorizontalAlign::Center,
                                },
                            ),
                            style: Style {
                                margin: Rect {
                                    top: Val::Px(32.0),
                                    ..default()
                                },
                                ..default()
                            },
                            ..default()
                        });
                });
        }

        #[derive(Component)]
        struct ScoreEntity;

        #[derive(Component)]
        struct Score(u32);

        fn update_ui(mut entities: Query<(&ScoreEntity, &mut Text, &Score), Changed<Score>>) {
            for (_, mut text, score) in entities.iter_mut() {
                for section in text.sections.iter_mut() {
                    section.value = score.0.to_string();
                }
            }
        }

        fn despawn_ui(mut commands: Commands, entities: Query<(Entity, &UiEntity)>) {
            for (entity, _) in entities.iter() {
                commands.entity(entity).despawn();
            }
        }

        #[derive(Component)]
        struct UiEntity;

        const BLOCK_SIZE: f32 = 48.0;
        const AREA_SIZE: (f32, f32) = (BLOCK_SIZE * 10.0, BLOCK_SIZE * 16.0);

        fn spawn_area(mut commands: Commands) {
            commands
                .spawn()
                .insert(AreaEntity)
                .insert_bundle(OrthographicCameraBundle::new_2d());

            commands
                .spawn()
                .insert(AreaEntity)
                .insert(AreaFieldEntity)
                .insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLACK,
                        custom_size: Some(Vec2::new(
                            AREA_SIZE.0 + BLOCK_SIZE,
                            AREA_SIZE.1 + BLOCK_SIZE,
                        )),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                });

            for y in 0..=16 {
                for x in [-1, 11].into_iter() {
                    let (x, y) = transform_as_in_area(x as f32, y as f32);

                    commands
                        .spawn()
                        .insert(MinoEntity)
                        .insert(DummyMinoEntity)
                        .insert_bundle(SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(0.0, 0.0)),
                                ..default()
                            },
                            transform: Transform::from_xyz(x, y, -1.0),
                            ..default()
                        });
                }
            }

            for x in -1..11 {
                let (x, y) = transform_as_in_area(x as f32, -1.0);

                commands
                    .spawn()
                    .insert(MinoEntity)
                    .insert(DummyMinoEntity)
                    .insert_bundle(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(0.0, 0.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, y, -1.0),
                        ..default()
                    });
            }

            spawn_falling(commands);
            // .with_children(|cb| {
            //     let (x, y) = transform_as_in_area(5.0, 16.0);
            //
            //     cb.spawn()
            //         .insert(FallingEntity)
            //         .insert_bundle(SpriteBundle {
            //             sprite: Sprite {
            //                 color: Color::GREEN,
            //                 custom_size: Some(Vec2::new(BLOCK_SIZE,
            // BLOCK_SIZE)),                 ..default()
            //             },
            //             transform: Transform::from_translation(Vec3::new(x,
            // y, 1.0)),             ..default()
            //         });
            // });
            // for (x, y) in [(0.0, 0.0), (10.0, 0.0), (0.0, 16.0), (5.0, 8.0),
            // (4.0, 9.0)] {     let (x, y) =
            // transform_as_in_area(x, y);     cb.spawn()
            //         .insert(AreaEntity)
            //         .insert(MinoEntity)
            //         .insert_bundle(SpriteBundle {
            //             sprite: Sprite {
            //                 color: Color::YELLOW_GREEN,
            //                 custom_size: Some(Vec2::new(BLOCK_SIZE,
            // BLOCK_SIZE)),                 ..default()
            //             },
            //             transform: Transform::from_translation(Vec3::new(x,
            // y, 1.0)),             ..default()
            //         });
            // }
        }

        fn transform_as_in_area(x: f32, y: f32) -> (f32, f32) {
            (
                BLOCK_SIZE * x - AREA_SIZE.0 / 2.0,
                BLOCK_SIZE * y - AREA_SIZE.1 / 2.0,
            )
        }

        fn untransform_as_in_area(x: f32, y: f32) -> (f32, f32) {
            (
                ((x + (AREA_SIZE.0 / 2.0)) / BLOCK_SIZE),
                ((y + (AREA_SIZE.1 / 2.0)) / BLOCK_SIZE),
            )
        }

        #[derive(Component)]
        struct AreaFieldEntity;

        #[derive(Component)]
        struct MinoEntity;

        #[derive(Component)]
        struct DummyMinoEntity;

        #[derive(Component)]
        struct FallingEntity;

        fn is_movable(
            entities: &Query<(&MinoEntity, &Transform), Without<FallingEntity>>,
            target: &Transform,
        ) -> bool {
            let [tx, ty, _] = target.translation.to_array();
            let (tx, ty) = untransform_as_in_area(tx, ty);

            for (_, transform) in entities.iter() {
                let [x, y, _] = transform.translation.to_array();
                let (x, y) = untransform_as_in_area(x, y);

                bevy::log::debug!("{} : {} | {} : {}", tx, x, ty, y); // magic code : slowing process?

                if tx.round() == x.round() && ty.round() == y.round() {
                    return false;
                }
            }

            true
        }

        fn tick_falling(
            mut stopwatch: Local<Stopwatch>,
            time: Res<Time>,
            mut entities: Query<(&FallingEntity, &mut Transform)>,
            minos: Query<(&MinoEntity, &Transform), Without<FallingEntity>>,
            mut landings: EventWriter<Landing>,
        ) {
            const THRESHOLD: f32 = 1.5;

            stopwatch.tick(time.delta());

            if stopwatch.elapsed_secs() < THRESHOLD {
                return;
            }

            stopwatch.reset();

            for (_, mut transform) in entities.iter_mut() {
                let [x, y, z] = transform.translation.to_array();

                let new_transform = transform.with_translation(Vec3::new(x, y - BLOCK_SIZE, z));

                if !is_movable(&minos, &new_transform) {
                    landings.send(Landing);
                    continue;
                }

                *transform = new_transform;
            }
        }

        struct Landing;

        fn p90_spin(mut transform: Transform) -> Transform {
            transform.rotate(Quat::from_rotation_z(std::f32::consts::PI / 2.0));
            transform
        }

        fn n90_spin(mut transform: Transform) -> Transform {
            transform.rotate(Quat::from_rotation_z(std::f32::consts::PI / -2.0));
            transform
        }

        // [+y]
        // ^
        // |
        // + ---> [+x]

        // [-- -- -- --]
        // [   ++      ]
        // [           ]
        // [           ]
        const I: [Transform; 4] = [
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(2.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
        ];

        // [--         ]
        // [-- ** --   ]
        // [           ]
        // [           ]
        const J: [Transform; 4] = [
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        // [      --   ]
        // [-- ** --   ]
        // [           ]
        // [           ]
        const L: [Transform; 4] = [
            Transform::from_xyz(1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        // [  -- --    ]
        // [  ** --    ]
        // [           ]
        // [           ]
        const O: [Transform; 4] = [
            Transform::from_xyz(0.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        // [   -- --   ]
        // [-- **      ]
        // [           ]
        // [           ]
        const S: [Transform; 4] = [
            Transform::from_xyz(0.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        // [   --      ]
        // [-- ** --   ]
        // [           ]
        // [           ]
        const T: [Transform; 4] = [
            Transform::from_xyz(0.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        // [-- --      ]
        // [   ** --   ]
        // [           ]
        // [           ]
        const Z: [Transform; 4] = [
            Transform::from_xyz(-1.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 1.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(0.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
            Transform::from_xyz(1.0 * BLOCK_SIZE, 0.0 * BLOCK_SIZE, 0.0),
        ];

        fn handle_landing(
            mut commands: Commands,
            landings: EventReader<Landing>,
            parents: Query<(Entity, &FallingEntity, &Children), With<FallingEntity>>,
            sprites: Query<(&Sprite, &GlobalTransform)>,
        ) {
            if landings.is_empty() {
                return;
            }

            for (parent, _, children) in parents.iter() {
                commands.entity(parent).despawn_recursive();
                for child in children.iter() {
                    let (sprite, current_transform) = sprites.get(*child).unwrap();
                    let sprite = sprite.clone();

                    commands
                        .spawn()
                        .insert(MinoEntity)
                        .insert_bundle(SpriteBundle {
                            sprite,
                            transform: (*current_transform).into(),
                            ..default()
                        });
                }
            }

            spawn_falling(commands);
        }

        fn spawn_falling(mut commands: Commands) {
            let (transforms, color) = match rand::random::<u8>() % 7 {
                0 => (I, Color::AQUAMARINE),
                1 => (J, Color::BLUE),
                2 => (L, Color::ORANGE),
                3 => (O, Color::YELLOW),
                4 => (S, Color::GREEN),
                5 => (T, Color::PINK),
                6 => (Z, Color::RED),
                _ => panic!(),
            };

            let (x, y) = transform_as_in_area(5.0, 16.0);

            commands
                .spawn()
                .insert(FallingEntity)
                .insert_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: Color::OLIVE,
                        custom_size: Some(Vec2::new(10.0, 10.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(x, y, 1.0),
                    ..default()
                })
                .with_children(|cb| {
                    for transform in transforms.into_iter() {
                        cb.spawn().insert_bundle(SpriteBundle {
                            transform,
                            sprite: Sprite {
                                color,
                                custom_size: Some(Vec2::new(BLOCK_SIZE, BLOCK_SIZE)),
                                ..default()
                            },
                            ..default()
                        });
                    }
                });
        }

        use crate::Config;
        enum FallingInput {
            Left,
            Right,
            HardDrop,
            P90Spin,
            N90Spin,
        }

        fn falling_input(
            key: Res<Input<KeyCode>>,
            mut inputs: EventWriter<FallingInput>,
            config: Res<Config>,
        ) {
            let config = &config.key.game;

            if key.just_pressed(config.left) {
                inputs.send(FallingInput::Left);
            } else if key.just_pressed(config.right) {
                inputs.send(FallingInput::Right);
            } else if key.just_pressed(config.hard_drop) {
                inputs.send(FallingInput::HardDrop);
            } else if key.just_pressed(config.p90_spin) {
                inputs.send(FallingInput::P90Spin);
            } else if key.just_pressed(config.n90_spin) {
                inputs.send(FallingInput::N90Spin);
            }
        }

        fn falling_handle(
            mut inputs: EventReader<FallingInput>,
            mut entities: Query<(&FallingEntity, &mut Transform)>,
            mut landings: EventWriter<Landing>,
            minos: Query<(&MinoEntity, &Transform), Without<FallingEntity>>,
        ) {
            for input in inputs.iter() {
                match *input {
                    FallingInput::Left =>
                        for (_, mut transform) in entities.iter_mut() {
                            let [x, y, z] = transform.translation.to_array();

                            let new_transform =
                                transform.with_translation(Vec3::new(x - BLOCK_SIZE, y, z));

                            if is_movable(&minos, &new_transform) {
                                *transform = new_transform;
                            }
                        },
                    FallingInput::Right =>
                        for (_, mut transform) in entities.iter_mut() {
                            let [x, y, z] = transform.translation.to_array();

                            let new_transform =
                                transform.with_translation(Vec3::new(x + BLOCK_SIZE, y, z));

                            if is_movable(&minos, &new_transform) {
                                *transform = new_transform;
                            }
                        },

                    FallingInput::HardDrop =>
                        for (_, mut transform) in entities.iter_mut() {
                            let [x, y, z] = transform.translation.to_array();
                            let (ux, mut uy) = untransform_as_in_area(x, y);

                            let mut new_transform;
                            loop {
                                let (x, y) = transform_as_in_area(ux, uy);
                                new_transform = transform.with_translation(Vec3::new(x, y, z));

                                if !is_movable(&minos, &new_transform) {
                                    break;
                                }

                                uy -= 1.0;
                            }
                            *transform = new_transform;

                            landings.send(Landing);
                        },

                    FallingInput::P90Spin =>
                        for (_, mut transform) in entities.iter_mut() {
                            *transform = p90_spin(*transform);
                        },
                    FallingInput::N90Spin =>
                        for (_, mut transform) in entities.iter_mut() {
                            *transform = n90_spin(*transform);
                        },
                };
            }
        }

        fn despawn_area(mut commands: Commands, entities: Query<(Entity, &AreaEntity)>) {
            for (entity, _) in entities.iter() {
                commands.entity(entity).despawn();
            }
        }

        #[derive(Component)]
        struct AreaEntity;
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
