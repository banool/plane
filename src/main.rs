/*
use bevy_rapier2d::{plugin::RapierPhysicsPlugin, rapier::prelude::PhysicsHooks, render::RapierDebugRenderPlugin};
use bevy::{log::LogPlugin, prelude::*, core::FrameCount, render::camera::ScalingMode};

struct Blah;

impl PhysicsHooks for Blah {}

const SKY_COLOR: Color = Color::rgb(0.69, 0.69, 0.69);

fn main() {
    let mut app = App::new();
    let rapier_physics_plugin: RapierPhysicsPlugin<()> = RapierPhysicsPlugin::default();
    app.insert_resource(ClearColor(SKY_COLOR))
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,matchbox_socket=debug".into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true, // behave on wasm
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(rapier_physics_plugin)
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, (setup, spawn_player))
        .init_resource::<FrameCount>()
        .run();
}

fn setup(mut commands: Commands) {
    let mut camera_bundle = Camera2dBundle::default();
    camera_bundle.projection.scaling_mode = ScalingMode::FixedVertical(10.);
    commands.spawn(camera_bundle);
}

fn spawn_player(mut commands: Commands) {
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0., 0.47, 1.),
            custom_size: Some(Vec2::new(1., 1.)),
            ..default()
        },
        ..default()
    });
}
*/


mod args;
mod box_game;

use bevy_rapier2d::{plugin::RapierPhysicsPlugin, rapier::prelude::PhysicsHooks};
use bevy::{log::LogPlugin, prelude::*, time::TimePlugin};
use bevy_ggrs::{GgrsPlugin, GgrsSchedule, ggrs::SessionBuilder, Session};
use bevy_matchbox::prelude::*;

use args::*;
use box_game::*;

struct Blah;

impl PhysicsHooks for Blah {}

const FPS: usize = 60;

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Lobby,
    InGame,
}

const SKY_COLOR: Color = Color::rgb(0.69, 0.69, 0.69);

fn main() {
    // read query string or command line arguments
    let args = Args::default();
    info!("{args:?}");

    let mut app = App::new();

    GgrsPlugin::<GgrsConfig>::new()
        // define frequency of rollback game logic update
        .with_update_frequency(FPS)
        // define system that returns inputs given a player handle, so GGRS can send the inputs
        // around
        .with_input_system(input)
        // register types of components AND resources you want to be rolled back
        .register_rollback_component::<Transform>()
        .register_rollback_component::<Velocity>()
        .register_rollback_resource::<FrameCount>()
        // make it happen in the bevy app
        .build(&mut app);

    let rapier_physics_plugin: RapierPhysicsPlugin<()> = RapierPhysicsPlugin::default();

    app.insert_resource(ClearColor(SKY_COLOR))
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,matchbox_socket=debug".into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(TimePlugin)
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true, // behave on wasm
                        ..default()
                    }),
                    ..default()
                }),

        )
        .add_plugins(rapier_physics_plugin)
        // Some of our systems need the query parameters
        .insert_resource(args)
        .init_resource::<FrameCount>()
        .add_state::<AppState>()
        .add_systems(OnEnter(AppState::Lobby), (lobby_startup, start_matchbox_socket))
        .add_systems(Update, lobby_system)
        .add_systems(OnExit(AppState::Lobby), lobby_cleanup)
        .add_systems(OnEnter(AppState::InGame), setup_scene_system)
        //.add_systems(Update, log_ggrs_events)
        // these systems will be executed as part of the advance frame update
        .add_systems(GgrsSchedule, (move_cube_system, increase_frame_system))
        .run();
}

fn start_matchbox_socket(mut commands: Commands, args: Res<Args>) {
    let room_id = match &args.room {
        Some(id) => id.clone(),
        None => format!("bevy_ggrs?next={}", &args.players),
    };

    let room_url = format!("{}/{}", &args.matchbox, room_id);
    info!("connecting to matchbox server: {room_url:?}");

    commands.insert_resource(MatchboxSocket::new_ggrs(room_url));
}

// Marker components for UI
#[derive(Component)]
struct LobbyText;

#[derive(Component)]
struct LobbyUI;

fn lobby_startup(mut commands: Commands) {
    commands.spawn(Camera3dBundle::default());

    // All this is just for spawning centered text.
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                ..default()
            },
            background_color: Color::rgb(0.43, 0.40, 0.38).into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: Style {
                        align_self: AlignSelf::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    text: Text::from_section(
                        "Entering lobby...",
                        TextStyle {
                            font: Default::default(),
                            font_size: 96.,
                            color: Color::BLACK,
                        },
                    ),
                    ..default()
                })
                .insert(LobbyText);
        })
        .insert(LobbyUI);
}

fn lobby_cleanup(query: Query<Entity, With<LobbyUI>>, mut commands: Commands) {
    for e in query.iter() {
        commands.entity(e).despawn_recursive();
    }
}

fn lobby_system(
    mut app_state: ResMut<NextState<AppState>>,
    args: Res<Args>,
    mut socket: ResMut<MatchboxSocket<SingleChannel>>,
    mut commands: Commands,
    mut query: Query<&mut Text, With<LobbyText>>,
) {
    // regularly call update_peers to update the list of connected peers
    for (peer, new_state) in socket.update_peers() {
        // you can also handle the specific dis(connections) as they occur:
        match new_state {
            PeerState::Connected => info!("peer {peer} connected"),
            PeerState::Disconnected => info!("peer {peer} disconnected"),
        }
    }

    let connected_peers = socket.connected_peers().count();
    let remaining = args.players - (connected_peers + 1);
    query.single_mut().sections[0].value = format!("Waiting for {remaining} more player(s)",);
    if remaining > 0 {
        return;
    }

    info!("All peers have joined, going in-game");

    // extract final player list
    let players = socket.players();

    let max_prediction = 12;

    // create a GGRS P2P session
    let mut sess_build = SessionBuilder::<GgrsConfig>::new()
        .with_num_players(args.players)
        .with_max_prediction_window(max_prediction)
        .with_input_delay(2)
        .with_fps(FPS)
        .expect("invalid fps");

    for (i, player) in players.into_iter().enumerate() {
        sess_build = sess_build
            .add_player(player, i)
            .expect("failed to add player");
    }

    let channel = socket.take_channel(0).unwrap();

    // start the GGRS session
    // todo, this is due to a deps mismatch
    let sess = sess_build
        .start_p2p_session(channel)
        .expect("failed to start session");

    commands.insert_resource(Session::P2P(sess));

    // transition to in-game state
    app_state.set(AppState::InGame);
}

fn log_ggrs_events(mut session: ResMut<Session<GgrsConfig>>) {
    match session.as_mut() {
        Session::P2P(s) => {
            for event in s.events() {
                info!("GGRS Event: {event:?}");
            }
        }
        _ => panic!("This example focuses on p2p."),
    }
}
