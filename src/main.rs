use bevy::prelude::*;
use bevy_ecs_tiled::prelude::*;
use bevy_quinnet::{
    server::{
        ConnectionLostEvent, EndpointAddrConfiguration, QuinnetServer, ServerEndpointConfiguration,
        certificate::CertificateRetrievalMode, endpoint::Endpoint,
    },
    shared::ClientId,
};
use bevy_replicon::prelude::*;
use bevy_replicon_quinnet::{ChannelsConfigurationExt, RepliconQuinnetPlugins};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const WALL_THICKNESS: f32 = 10.0;
// x coordinates
const LEFT_WALL: f32 = -450.;
const RIGHT_WALL: f32 = 450.;
// y coordinates
const BOTTOM_WALL: f32 = -300.;

const PADDLE_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.
const PADDLE_SIZE: Vec2 = Vec2::new(120.0, 20.0);
const GAP_BETWEEN_PADDLE_AND_FLOOR: f32 = 60.0;
const PADDLE_SPEED: f32 = 500.0;
// How close can the paddle get to the wall
const PADDLE_PADDING: f32 = 10.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TiledPlugin::default())
        .add_plugins(RepliconPlugins)
        .add_plugins(RepliconQuinnetPlugins)
        .add_plugins(HelloPlugin)
        .insert_resource(Users::default())
        .add_systems(Startup, (startup, start_listening))
        .add_systems(Update, (handle_client_messages, handle_server_events))
        .add_systems(FixedUpdate, move_player)
        .run();
}

fn startup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a 2D camera
    commands.spawn(Camera2d);

    // Load a map asset and retrieve its handle
    let map_handle: Handle<TiledMapAsset> = asset_server.load("untitled.tmx");

    // Spawn a new entity with the TiledMap component
    commands.spawn((TiledMap(map_handle), TilemapAnchor::Center));

    // Paddle
    let paddle_y = BOTTOM_WALL + GAP_BETWEEN_PADDLE_AND_FLOOR;

    commands.spawn((
        Sprite::from_color(PADDLE_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(0.0, paddle_y, 0.0),
            scale: PADDLE_SIZE.extend(1.0),
            ..default()
        },
        Paddle,
    ));
}

#[derive(Resource)]
struct GreetTimer(Timer);

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

#[derive(Resource, Debug, Clone, Default)]
struct Users {
    names: HashMap<ClientId, String>,
}

#[derive(Component)]
struct Paddle;

// Messages from clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join { name: String },
    Disconnect {},
    ChatMessage { message: String },
}

// Messages from the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    ClientConnected {
        client_id: ClientId,
        username: String,
    },
    ClientDisconnected {
        client_id: ClientId,
    },
    ChatMessage {
        client_id: ClientId,
        message: String,
    },
    InitClient {
        client_id: ClientId,
        usernames: HashMap<ClientId, String>,
    },
}

pub struct HelloPlugin;

impl Plugin for HelloPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GreetTimer(Timer::from_seconds(2.0, TimerMode::Repeating)));

        app.add_systems(Startup, (add_people, update_people).chain());
        app.add_systems(Update, (hello_world, greet_people));
    }
}

fn add_people(mut commands: Commands) {
    commands.spawn((Person, Name("Elaina Proctor".to_string())));
    commands.spawn((Person, Name("Renzo Hume".to_string())));
    commands.spawn((Person, Name("Zayna Nieves".to_string())));
}

fn greet_people(time: Res<Time>, mut timer: ResMut<GreetTimer>, query: Query<&Name, With<Person>>) {
    // update our timer with the time elapsed since the last update
    // if that caused the timer to finish, we say hello to everyone
    if timer.0.tick(time.delta()).just_finished() {
        for name in &query {
            println!("hello {}!", name.0);
        }
    }
}

fn update_people(mut query: Query<&mut Name, With<Person>>) {
    for mut name in &mut query {
        if name.0 == "Elaina Proctor" {
            name.0 = "Elaina Hume".to_string();
            break; // We don't need to change any other names.
        }
    }
}

fn hello_world(timer: Res<GreetTimer>) {
    if timer.0.just_finished() {
        println!("hello world!");
    }
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut paddle_transform: Single<&mut Transform, With<Paddle>>,
    time: Res<Time>,
) {
    let mut direction = 0.0;

    if keyboard_input.pressed(KeyCode::KeyK) {
        direction -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::KeyJ) {
        direction += 1.0;
    }

    // Calculate the new horizontal paddle position based on player input
    let new_paddle_position =
        paddle_transform.translation.x + direction * PADDLE_SPEED * time.delta_secs();

    // Update the paddle position,
    // making sure it doesn't cause the paddle to leave the arena
    let left_bound = LEFT_WALL + WALL_THICKNESS / 2.0 + PADDLE_SIZE.x / 2.0 + PADDLE_PADDING;
    let right_bound = RIGHT_WALL - WALL_THICKNESS / 2.0 - PADDLE_SIZE.x / 2.0 - PADDLE_PADDING;

    paddle_transform.translation.x = new_paddle_position.clamp(left_bound, right_bound);
}

// fn start_listening(mut server: ResMut<QuinnetServer>) {
//     server.start_endpoint(ServerEndpointConfiguration {
//         addr_config: EndpointAddrConfiguration::from_ip(Ipv6Addr::UNSPECIFIED, 6000),
//         cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
//             server_hostname: Ipv6Addr::LOCALHOST.to_string(),
//         },
//         defaultables: Default::default(),
//     });
// }

fn handle_client_messages(mut server: ResMut<QuinnetServer>, mut users: ResMut<Users>) {
    let endpoint = server.endpoint_mut();
    for client_id in endpoint.clients() {
        while let Some(message) = endpoint.try_receive_message(client_id) {
            match message {
                ClientMessage::Join { name } => {
                    if let std::collections::hash_map::Entry::Vacant(e) =
                        users.names.entry(client_id)
                    {
                        info!("{} connected", name);
                        e.insert(name.clone());
                        // Initialize this client with existing state
                        endpoint
                            .send_message(
                                client_id,
                                ServerMessage::InitClient {
                                    client_id,
                                    usernames: users.names.clone(),
                                },
                            )
                            .unwrap();
                        // Broadcast the connection event
                        endpoint
                            .send_group_message(
                                users.names.keys(),
                                ServerMessage::ClientConnected {
                                    client_id,
                                    username: name,
                                },
                            )
                            .unwrap();
                    } else {
                        warn!(
                            "Received a Join from an already connected client: {}",
                            client_id
                        )
                    }
                }
                ClientMessage::Disconnect {} => {
                    // Tell the server endpoint to disconnect this user
                    endpoint.disconnect_client(client_id).unwrap();
                    handle_disconnect(endpoint, &mut users, client_id);
                }
                ClientMessage::ChatMessage { message } => {
                    info!(
                        "Chat message | {:?}: {}",
                        users.names.get(&client_id),
                        message
                    );
                    endpoint.try_send_group_message(
                        users.names.keys(),
                        ServerMessage::ChatMessage { client_id, message },
                    );
                }
            }
        }
    }
}

fn handle_server_events(
    mut connection_lost_events: MessageReader<ConnectionLostEvent>,
    mut server: ResMut<QuinnetServer>,
    mut users: ResMut<Users>,
) {
    // The server signals us about users that lost connection
    for client in connection_lost_events.read() {
        handle_disconnect(server.endpoint_mut(), &mut users, client.id);
    }
}

/// Shared disconnection behaviour, whether the client lost connection or asked to disconnect
fn handle_disconnect(endpoint: &mut Endpoint, users: &mut ResMut<Users>, client_id: ClientId) {
    // Remove this user
    if let Some(username) = users.names.remove(&client_id) {
        // Broadcast its deconnection

        endpoint
            .send_group_message(
                users.names.keys(),
                ServerMessage::ClientDisconnected { client_id },
            )
            .unwrap();
        info!("{} disconnected", username);
    } else {
        warn!(
            "Received a Disconnect from an unknown or disconnected client: {}",
            client_id
        )
    }
}

fn start_listening(mut server: ResMut<QuinnetServer>) {
    server
        .start_endpoint(ServerEndpointConfiguration {
            addr_config: EndpointAddrConfiguration::from_string("[::]:6000").unwrap(),
            cert_mode: CertificateRetrievalMode::GenerateSelfSigned {
                server_hostname: "::1".to_owned(),
            },
            defaultables: Default::default(),
        })
        .unwrap();
}
