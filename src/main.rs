// simple flappy bird game in rust using bevy game engine.
// :)

use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, Image, ImageSampler, ImageType};
use bevy::prelude::*; 
use rand::Rng;

// basic game config values for physics and spacing stuff
const GRAVITY: f32 = -500.0;
const JUMP_FORCE: f32 = 250.0;
const PIPE_SPEED: f32 = 200.0;
const PIPE_SPAWN_X: f32 = 500.0;
const PIPE_WIDTH: f32 = 80.0;
const PIPE_HEIGHT: f32 = 420.0;
const PIPE_GAP: f32 = 220.0;
const PIPE_OFFSET_LIMIT: f32 = 170.0;
const PIPE_DESPAWN_X: f32 = -600.0;
const BIRD_WIDTH: f32 = 34.0 * 0.1;
const BIRD_HEIGHT: f32 = 24.0 * 0.1;
const FLOOR_Y: f32 = -350.0;

// ecs components and data structs for state tracking
#[derive(Component)]
struct Bird;

#[derive(Component)]
struct Velocity(f32);

#[derive(Component)]
struct GameOverScreen;

#[derive(Resource)]
struct GameStarted(bool);

#[derive(Resource)]
struct GameOver(bool);

#[derive(Component)]
struct Pipe;

#[derive(Resource)]
struct PipeTimer(Timer);


// setting up bevy app instance and registering all game systems to update loop fr
fn main() { 
    App::new() 
        .insert_resource(GameStarted(false)) 
        .insert_resource(GameOver(false)) 
        .insert_resource(PipeTimer(Timer::from_seconds(2.0, TimerMode::Repeating))) 
        .add_plugins(DefaultPlugins) 
        .add_systems(Startup, setup) 
        .add_systems(
            Update,
            ( 
                jump_system,
                gravity_system,
                move_pipe_system,
                spawn_pipes,
                collision_system,
                game_over_system,
            ),
        )
        .run(); 
}

// setup system to spawn 2d camera view, background image, and the initial player bird entity
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
) {
    let bird_image = Image::from_buffer(
        include_bytes!("../assets/bird.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .expect("embedded bird sprite should decode");
    let bird_handle = images.add(bird_image);

    commands.spawn(Camera2d); 

    commands.spawn(( 
        Sprite {
            image: asset_server.load("background-day.png"),
            custom_size: Some(Vec2::new(1200.0, 800.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.0),
    ));

    commands.spawn(( 
        Sprite::from_image(bird_handle),
        Transform {
            scale: Vec3::splat(0.1),
            translation: Vec3::new(0.0, 100.0, 0.0),
            ..default()
        },
        Bird,
        Velocity(0.0),
    ));

    spawn_pipe_pair(&mut commands, 0.0);
}

// handles keyboard space hits to trigger jumps if player isnt cooked already
fn jump_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut started: ResMut<GameStarted>,
    game_over: Res<GameOver>,
    mut query: Query<&mut Velocity, With<Bird>>,
) {
    if game_over.0 {
        return;
    }

    if keyboard.just_pressed(KeyCode::Space) {
        started.0 = true;

        if let Ok(mut velocity) = query.single_mut() {
            velocity.0 = JUMP_FORCE;
        }
    }
}

// apply gravity constant to velocity using delta time frames
fn gravity_system(
    time: Res<Time>,
    started: Res<GameStarted>,
    game_over: Res<GameOver>,
    mut query: Query<(&mut Transform, &mut Velocity), With<Bird>>,
) {
    if !started.0 || game_over.0 {
        return;
    }

    if let Ok((mut transform, mut velocity)) = query.single_mut() {
        velocity.0 += GRAVITY * time.delta_secs();
        transform.translation.y += velocity.0 * time.delta_secs();
    }
}

// checks if bird fall past floor threshold to flip the game over status state
fn game_over_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut game_over: ResMut<GameOver>,
    bird_query: Query<&Transform, With<Bird>>,
    overlay_query: Query<Entity, With<GameOverScreen>>,
) {
    if game_over.0 {
        show_game_over_overlay(&mut commands, &asset_server, &overlay_query);
        return;
    }

    let Ok(transform) = bird_query.single() else {
        return;
    };

    if transform.translation.y < FLOOR_Y {
        game_over.0 = true;
        show_game_over_overlay(&mut commands, &asset_server, &overlay_query);
    }
}

// loops thru all active pipes to shift them leftward and despawns if past offscreen edge
fn move_pipe_system(
    mut commands: Commands,
    time: Res<Time>,
    started: Res<GameStarted>,
    game_over: Res<GameOver>,
    mut pipes: Query<(Entity, &mut Transform), With<Pipe>>,
) {
    if !started.0 || game_over.0 {
        return;
    }

    for (entity, mut pipe) in &mut pipes {
        pipe.translation.x -= PIPE_SPEED * time.delta_secs();

        if pipe.translation.x < PIPE_DESPAWN_X {
            commands.entity(entity).despawn();
        }
    }
}

// ticks the generation timer and uses rand rng range for random height offsets
fn spawn_pipes(
    mut commands: Commands,
    time: Res<Time>,
    started: Res<GameStarted>,
    game_over: Res<GameOver>,
    mut timer: ResMut<PipeTimer>,
) {
    if !started.0 || game_over.0 {
        return;
    }

    timer.0.tick(time.delta());

    if timer.0.just_finished() {
        let mut rng = rand::rng();
        let y = rng.random_range(-PIPE_OFFSET_LIMIT..PIPE_OFFSET_LIMIT);
        spawn_pipe_pair(&mut commands, y);
    }
}

// manual AABB bounding box checks for intersection detection, lowkey standard physics loop, AABB means axis aligned bounding box
fn collision_system(
    mut game_over: ResMut<GameOver>,
    bird_query: Query<&Transform, With<Bird>>,
    pipe_query: Query<&Transform, With<Pipe>>,
) {
    if game_over.0 {
        return;
    }

    let Ok(bird) = bird_query.single() else {
        return;
    };

    for pipe in &pipe_query {
        let dx = (bird.translation.x - pipe.translation.x).abs();
        let dy = (bird.translation.y - pipe.translation.y).abs();

        let hit_x = dx < (BIRD_WIDTH + PIPE_WIDTH) / 2.0;
        let hit_y = dy < (BIRD_HEIGHT + PIPE_HEIGHT) / 2.0;

        if hit_x && hit_y {
            game_over.0 = true;
            break;
        }
    }
}

// helper math function to split top and bottom positions for pipe pairs
fn spawn_pipe_pair(commands: &mut Commands, center_y: f32) {
    let top_y = center_y + PIPE_GAP / 2.0 + PIPE_HEIGHT / 2.0;
    let bottom_y = center_y - PIPE_GAP / 2.0 - PIPE_HEIGHT / 2.0;

    for y in [top_y, bottom_y] {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.0, 1.0, 0.0), Vec2::new(PIPE_WIDTH, PIPE_HEIGHT)),
            Transform::from_xyz(PIPE_SPAWN_X, y, 0.0),
            Pipe,
        ));
    }
}

// spawns the gameover UI graphic overlay if it dont exist on screen yet
fn show_game_over_overlay(
    commands: &mut Commands,
    asset_server: &AssetServer,
    overlay_query: &Query<Entity, With<GameOverScreen>>,
) {
    if overlay_query.iter().next().is_some() {
        return;
    }

    commands.spawn((
        Sprite::from_image(asset_server.load("gameover.png")),
        Transform {
            scale: Vec3::splat(0.5),
            translation: Vec3::new(0.0, 0.0, 10.0),
            ..default()
        },
        GameOverScreen,
    ));
}
