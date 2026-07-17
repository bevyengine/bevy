//! A simple minesweeper-style game in Bevy UI.

use bevy::color::palettes::css::{DARK_GREEN, LIME, ORANGE, ORANGE_RED, RED, SEA_GREEN, YELLOW};
use bevy::prelude::*;
use bevy::text::LineHeight;
use rand::seq::SliceRandom;
use std::ops::Range;

const TILE_SIZE: f32 = 30.0;
const TILE_GAP: f32 = 6.0;
const BOARD_PADDING: f32 = 15.0;

const BACKGROUND_COLOR: Color = Color::Srgba(DARK_GREEN);
const TILE_BORDER_COLOR: Color = Color::Srgba(SEA_GREEN);
const HOVERED_TILE_BORDER_COLOR: Color = Color::Srgba(LIME);
const MILD_DANGER_COLOR: Color = Color::Srgba(YELLOW);
const SOME_DANGER_COLOR: Color = Color::Srgba(ORANGE);
const VERY_DANGER_COLOR: Color = Color::Srgba(ORANGE_RED);
const MOST_DANGER_COLOR: Color = Color::Srgba(RED);

#[derive(Default, Clone, Copy)]
struct Tile {
    mined: bool,
    revealed: bool,
    flagged: bool,
}

struct MineField {
    width: i32,
    height: i32,
    tiles: Vec<Tile>,
}

impl MineField {
    fn xs(&self) -> Range<i32> {
        0..self.width
    }

    fn ys(&self) -> Range<i32> {
        0..self.height
    }

    fn get(&self, p: IVec2) -> Option<usize> {
        if self.xs().contains(&p.x) && self.ys().contains(&p.y) {
            Some((p.y * self.width + p.x) as usize)
        } else {
            None
        }
    }

    fn new(width: i32, height: i32, mines: usize) -> Self {
        let mut tiles = vec![Tile::default(); (width * height) as usize];
        tiles
            .iter_mut()
            .take(mines)
            .for_each(|tile| tile.mined = true);

        tiles.shuffle(&mut rand::rng());

        Self {
            width,
            height,
            tiles,
        }
    }

    fn get_adjacent(&self, target: IVec2) -> impl Iterator<Item = IVec2> + '_ {
        [
            (1, 1),
            (1, 0),
            (1, -1),
            (0, 1),
            (0, -1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ]
        .map(|d| target + IVec2::from(d))
        .into_iter()
        .filter(|adjacent| self.get(*adjacent).is_some())
    }

    fn count_adjacent_mines(&self, target: IVec2) -> usize {
        self.get_adjacent(target)
            .filter(|&adjacent| self[adjacent].mined)
            .count()
    }

    fn reveal(&mut self, target: IVec2) {
        let mut open = vec![target];
        while let Some(current) = open.pop() {
            if !self[current].revealed {
                self[current].revealed = true;
                if self.count_adjacent_mines(current) == 0 {
                    open.extend(self.get_adjacent(current));
                }
            }
        }
    }

    fn iter(&self) -> impl Iterator<Item = (IVec2, Tile)> + '_ {
        self.ys()
            .flat_map(|y| self.xs().map(move |x| IVec2::new(x, y)))
            .map(|tile| (tile, self[tile]))
    }

    fn flag_count(&self) -> usize {
        self.tiles.iter().filter(|tile| tile.flagged).count()
    }

    fn mine_count(&self) -> usize {
        self.tiles.iter().filter(|tile| tile.mined).count()
    }

    fn is_cleared(&self) -> bool {
        self.tiles
            .iter()
            .all(|tile| tile.revealed && !tile.mined || tile.mined && tile.flagged)
    }
}

impl std::ops::Index<IVec2> for MineField {
    type Output = Tile;

    fn index(&self, tile: IVec2) -> &Self::Output {
        let index = self.get(tile).unwrap();
        &self.tiles[index]
    }
}

impl std::ops::IndexMut<IVec2> for MineField {
    fn index_mut(&mut self, tile: IVec2) -> &mut Self::Output {
        let index = self.get(tile).unwrap();
        &mut self.tiles[index]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    fn field(self) -> MineField {
        match self {
            Difficulty::Easy => MineField::new(15, 10, 15),
            Difficulty::Normal => MineField::new(20, 15, 50),
            Difficulty::Hard => MineField::new(25, 15, 75),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Menu,
    Playing,
}

#[derive(Resource)]
struct Game {
    field: MineField,
    game_over: bool,
}

impl Default for Game {
    fn default() -> Self {
        Self {
            field: Difficulty::Normal.field(),
            game_over: false,
        }
    }
}

#[derive(Component)]
struct GameRoot;

#[derive(Component)]
struct TileCell(IVec2);

#[derive(Component, Clone, Copy)]
enum ButtonAction {
    NewGame(Difficulty),
    Menu,
}

fn text_button(label: &'static str, action: ButtonAction) -> impl Bundle {
    (
        action,
        Node {
            min_width: px(144),
            border: px(3).all(),
            border_radius: px(8).into(),
            box_sizing: BoxSizing::ContentBox,
            ..default()
        },
        BorderColor::all(TILE_BORDER_COLOR),
        BackgroundColor(BACKGROUND_COLOR),
        Text::new(label),
        TextFont::from_font_size(24.0),
        TextLayout::justify(Justify::Center),
        LineHeight::Px(40.),
    )
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_menu(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::Menu),
        Node {
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: px(20),
            ..default()
        },
        children![
            (Text::new("Bevy Mines"), TextFont::from_font_size(35.0)),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(10),
                    ..default()
                },
                children![
                    text_button("easy", ButtonAction::NewGame(Difficulty::Easy)),
                    text_button("normal", ButtonAction::NewGame(Difficulty::Normal)),
                    text_button("hard", ButtonAction::NewGame(Difficulty::Hard)),
                ],
            ),
        ],
    ));
}

fn setup_game(mut commands: Commands, game: Res<Game>, assets: Res<AssetServer>) {
    let root = commands
        .spawn((
            GameRoot,
            DespawnOnExit(GameState::Playing),
            Node {
                margin: auto().all(),
                ..default()
            },
        ))
        .id();

    rebuild_game_ui(&mut commands, root, &game, &assets);
}

fn rebuild_game_ui(commands: &mut Commands, root: Entity, game: &Game, assets: &AssetServer) {
    commands.entity(root).despawn_related::<Children>();

    commands.entity(root).with_children(|parent| {
        parent
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(20),
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            display: Display::Grid,
                            padding: UiRect::all(px(BOARD_PADDING)),
                            border_radius: BorderRadius::all(px(25)),
                            row_gap: px(TILE_GAP),
                            column_gap: px(TILE_GAP),
                            grid_template_columns: RepeatedGridTrack::px(
                                game.field.width,
                                TILE_SIZE,
                            ),
                            grid_template_rows: RepeatedGridTrack::px(game.field.height, TILE_SIZE),
                            ..default()
                        },
                        BackgroundColor(BACKGROUND_COLOR),
                    ))
                    .with_children(|parent| {
                        for (position, tile) in game.field.iter() {
                            let tile_node = Node {
                                grid_column: GridPlacement::start((position.x + 1) as i16),
                                grid_row: GridPlacement::start((position.y + 1) as i16),
                                border: px(4).all(),
                                border_radius: px(8).into(),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            };

                            if game.game_over && tile.mined {
                                parent.spawn((
                                    tile_node,
                                    BackgroundColor(BACKGROUND_COLOR),
                                    BorderColor::all(Color::NONE),
                                    Pickable::IGNORE,
                                    children![(
                                        Text::new("*"),
                                        TextFont::from_font_size(24.0),
                                        TextColor(MOST_DANGER_COLOR.into()),
                                    )],
                                ));
                            } else if game.game_over || tile.revealed {
                                let mine_count = game.field.count_adjacent_mines(position);
                                let color = match mine_count {
                                    0 => continue,
                                    1 => MILD_DANGER_COLOR,
                                    2 => SOME_DANGER_COLOR,
                                    3 => VERY_DANGER_COLOR,
                                    _ => MOST_DANGER_COLOR,
                                };
                                parent.spawn((
                                    tile_node,
                                    BackgroundColor(BACKGROUND_COLOR),
                                    BorderColor::all(color),
                                    Pickable::IGNORE,
                                    children![(
                                        Text::new(mine_count.to_string()),
                                        TextFont::from_font_size(20.0),
                                        TextColor(color),
                                    )],
                                ));
                            } else {
                                parent
                                    .spawn((
                                        tile_node,
                                        TileCell(position),
                                        BackgroundColor(BACKGROUND_COLOR),
                                        BorderColor::all(TILE_BORDER_COLOR),
                                    ))
                                    .with_children(|parent| {
                                        if tile.flagged {
                                            parent.spawn((
                                                ImageNode::new(assets.load("flag.png")),
                                                Node {
                                                    width: px(20),
                                                    height: px(20),
                                                    ..default()
                                                },
                                                Pickable::IGNORE,
                                            ));
                                        }
                                    });
                            }
                        }
                    });

                parent
                    .spawn(Node {
                        width: percent(100),
                        height: px(50),
                        margin: px(15).vertical(),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn((
                            Node {
                                align_items: AlignItems::Center,
                                column_gap: px(3),
                                ..default()
                            },
                            children![
                                (
                                    ImageNode::new(assets.load("flag.png")),
                                    Node {
                                        width: px(28),
                                        height: px(28),
                                        ..default()
                                    },
                                    Pickable::IGNORE,
                                ),
                                (
                                    Text::new(
                                        (game.field.mine_count() - game.field.flag_count())
                                            .to_string()
                                    ),
                                    TextFont::from_font_size(24.0),
                                )
                            ],
                        ));

                        if game.field.is_cleared() || game.game_over {
                            parent.spawn((
                                Node {
                                    width: percent(100),
                                    height: percent(100),
                                    position_type: PositionType::Absolute,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                Pickable::IGNORE,
                                children![(
                                    Text::new(if game.field.is_cleared() {
                                        "mines cleared!"
                                    } else {
                                        "boom!"
                                    }),
                                    TextFont::from_font_size(24.0),
                                )],
                            ));

                            parent.spawn(text_button("new game", ButtonAction::Menu));
                        }
                    });
            });
    });
}

fn on_button_click(
    click: On<Pointer<Click>>,
    buttons: Query<&ButtonAction>,
    mut game: ResMut<Game>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if click.button != PointerButton::Primary {
        return;
    }

    let Ok(action) = buttons.get(click.event_target()) else {
        return;
    };

    match *action {
        ButtonAction::NewGame(difficulty) => {
            game.field = difficulty.field();
            game.game_over = false;
            next_state.set(GameState::Playing);
        }
        ButtonAction::Menu => {
            next_state.set(GameState::Menu);
        }
    }
}

fn on_button_over(
    over: On<Pointer<Over>>,
    mut buttons: Query<(&mut BorderColor, &mut BackgroundColor), With<ButtonAction>>,
) {
    if let Ok((mut border, mut background)) = buttons.get_mut(over.event_target()) {
        border.set_all(HOVERED_TILE_BORDER_COLOR);
        background.0 = BACKGROUND_COLOR;
    }
}

fn on_button_out(
    out: On<Pointer<Out>>,
    mut buttons: Query<(&mut BorderColor, &mut BackgroundColor), With<ButtonAction>>,
) {
    if let Ok((mut border, mut background)) = buttons.get_mut(out.event_target()) {
        border.set_all(TILE_BORDER_COLOR);
        background.0 = BACKGROUND_COLOR;
    }
}

fn on_tile_over(over: On<Pointer<Over>>, mut tiles: Query<&mut BorderColor, With<TileCell>>) {
    if let Ok(mut border) = tiles.get_mut(over.event_target()) {
        border.set_all(HOVERED_TILE_BORDER_COLOR);
    }
}

fn on_tile_out(out: On<Pointer<Out>>, mut tiles: Query<&mut BorderColor, With<TileCell>>) {
    if let Ok(mut border) = tiles.get_mut(out.event_target()) {
        border.set_all(TILE_BORDER_COLOR);
    }
}

fn on_tile_click(
    click: On<Pointer<Click>>,
    tiles: Query<&TileCell>,
    mut game: ResMut<Game>,
    mut commands: Commands,
    game_root: Single<Entity, With<GameRoot>>,
    assets: Res<AssetServer>,
) {
    if game.game_over || game.field.is_cleared() {
        return;
    }

    let Ok(&TileCell(position)) = tiles.get(click.event_target()) else {
        return;
    };

    match click.button {
        PointerButton::Primary => {
            if game.field[position].mined {
                game.game_over = true;
            } else {
                game.field.reveal(position);
            }
        }
        PointerButton::Secondary => {
            if game.field[position].flagged {
                game.field[position].flagged = false;
            } else if game.field.flag_count() < game.field.mine_count() {
                game.field[position].flagged = true;
            }
        }
        _ => {
            return;
        }
    }

    rebuild_game_ui(&mut commands, *game_root, &game, &assets);
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Game>()
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(OnEnter(GameState::Playing), setup_game)
        .add_observer(on_button_click)
        .add_observer(on_button_over)
        .add_observer(on_button_out)
        .add_observer(on_tile_click)
        .add_observer(on_tile_over)
        .add_observer(on_tile_out)
        .run();
}
