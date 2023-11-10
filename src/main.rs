use bevy::prelude::*;

fn main() {
    App::new()
        .add_systems(Startup, add_people)
        .add_systems(Update, greet_things_with_names)
        .run();
}

// component
#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Person;

#[derive(Component)]
struct Name(String);

// system
fn greet_things_with_names(query: Query<&Name>) {
    for thing in &query {
        println!("Hi {}", thing.0);
    }
}

fn add_people(mut commands: Commands) {
    commands.spawn((Person, Name("Elaina Proctor".to_string())));
    commands.spawn((Person, Name("Renzo Hume".to_string())));
    commands.spawn((Person, Name("Zayna Nieves".to_string())));
}
