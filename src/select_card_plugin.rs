use crate::{AppConfig, GameState};
use bevy::prelude::*;

#[derive(Component, Reflect)]
struct SelectCardUi;

#[derive(Component, Reflect)]
struct MoveFaster;

pub(crate) struct SelectCardPlugin;

impl Plugin for SelectCardPlugin {
    fn build(&self, app: &mut App) {
        // This stage allows both players to pick a card. The card modifies various abilities of the player.
        app.add_systems(OnEnter(GameState::PickCard), setup);
        app.add_systems(
            Update,
            (switch_state_on_space, button_system).run_if(in_state(GameState::PickCard)),
        );
        app.add_systems(OnExit(GameState::PickCard), teardown);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                ..Default::default()
            },
            SelectCardUi,
        ))
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        height: Val::Px(100.),
                        width: Val::Px(200.),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..Default::default()
                })
                .with_children(|button| {
                    button.spawn(TextBundle::from_section(
                        "Move Faster",
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ));
                });

            parent
                .spawn(ButtonBundle {
                    style: Style {
                        height: Val::Px(100.),
                        width: Val::Px(200.),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: NORMAL_BUTTON.into(),
                    ..Default::default()
                })
                .with_children(|button| {
                    button.spawn(TextBundle::from_section(
                        "More Damage",
                        TextStyle {
                            font_size: 20.,
                            color: Color::WHITE,
                            ..Default::default()
                        },
                    ));
                });
        });
}

fn teardown(mut commands: Commands, ui_elements: Query<Entity, With<SelectCardUi>>) {
    ui_elements.for_each(|e| {
        commands.entity(e).despawn_recursive();
    })
}

fn switch_state_on_space(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.set(GameState::Round);
    }
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

fn button_system(
    mut commands: Commands,
    config: Res<AppConfig>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
    players: Query<(Entity, &crate::Player)>,
    mut state: ResMut<NextState<GameState>>,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let player = players
                    .iter()
                    .find(|(_, p)| p.client_id == config.client_id);

                match player {
                    None => println!("We could not find ourselves: {}", config.client_id),
                    Some((entity, _)) => {
                        commands.entity(entity).with_children(|parent| {
                            parent.spawn(MoveFaster);
                        });
                        state.set(GameState::Round);
                    }
                };

                *color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
