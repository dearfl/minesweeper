use bevy::{
    color::palettes::{
        css::{GREEN, RED, WHITE},
        tailwind::{RED_200, RED_400, RED_600, RED_800, RED_900, SKY_300, SKY_400},
    },
    core_pipeline::bloom::Bloom,
    ecs::{query::QueryData, system::SystemParam},
    input::common_conditions::input_just_released,
    prelude::*,
};
use rand::seq::SliceRandom;

const X: i32 = 30;
const Y: i32 = 16;
const BOMBS: i32 = 70;
const UNIT: f32 = 48.0;
const GAP: f32 = 2.0;
const PADDING: f32 = 24.0;
pub const SCREEN_WIDTH: f32 = X as f32 * UNIT + (X - 1) as f32 * GAP + PADDING * 2.0;
pub const SCREEN_HEIGHT: f32 = Y as f32 * UNIT + (Y - 1) as f32 * GAP + PADDING * 2.0;

#[derive(Clone, Debug, Resource)]
pub struct Materials {
    covered: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,

    // TODO: replace these materials to give better visual
    flagged: Handle<ColorMaterial>,
    bomb: Handle<ColorMaterial>,
    count: [Handle<ColorMaterial>; 8],
}

impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        let covered = mats.add(Color::from(SKY_400));
        let hovered = mats.add(Color::from(SKY_300));
        let flagged = mats.add(Color::from(GREEN));
        let bomb = mats.add(Color::from(RED));
        let count = [
            mats.add(Color::from(WHITE)),
            mats.add(Color::from(RED_200)),
            mats.add(Color::from(RED_400)),
            mats.add(Color::from(RED_600)),
            mats.add(Color::from(RED_800)),
            mats.add(Color::from(RED_900)),
            mats.add(Color::from(RED_900)),
            mats.add(Color::from(RED_900)),
        ];
        Self {
            covered,
            hovered,
            flagged,
            bomb,
            count,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, States, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Prepare,
    Running,
    Over,
}

impl GameState {
    pub fn is_running(&self) -> bool {
        matches!(self, GameState::Running)
    }
}

// a covered cell can be uncovered or flagged
#[derive(Clone, Copy, Debug, Component)]
pub struct Covered;

#[derive(Clone, Copy, Debug, Component)]
pub struct Flagged;

#[derive(Clone, Copy, Debug, Component)]
#[require(Transform, Visibility)]
pub struct Cell {
    x: i32,
    y: i32,
    is_bomb: bool,
}

#[derive(Clone, Debug)]
pub struct Board {
    columns: i32,
    rows: i32,
    _bombs: i32,
    grids: Vec<Cell>,
}

impl Board {
    pub fn new(columns: i32, rows: i32, bombs: i32) -> Self {
        let mut rng = rand::rng();
        let mut grids: Vec<bool> = (0..(columns * rows)).map(|idx| idx < bombs).collect();
        grids.shuffle(&mut rng);
        let grids = grids
            .iter()
            .enumerate()
            .map(|(idx, &is_bomb)| Cell {
                x: idx as i32 % columns,
                y: idx as i32 / columns,
                is_bomb,
            })
            .collect();
        Self {
            columns,
            rows,
            _bombs: bombs,
            grids,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Cell> {
        self.grids.iter().copied()
    }
}

#[derive(QueryData)]
#[query_data(mutable, derive(Debug))]
pub struct BoardQuery {
    entity: Entity,
    material: &'static mut MeshMaterial2d<ColorMaterial>,
    cell: &'static Cell,
    covered: Option<&'static Covered>,
    flagged: Option<&'static Flagged>,
}

#[derive(SystemParam)]
pub struct InterationParam<'w, 's> {
    query: Query<'w, 's, BoardQuery>,
    command: Commands<'w, 's>,
    materials: Res<'w, Materials>,
}

impl InterationParam<'_, '_> {
    fn count_adjacents(&self, target: Entity) -> Result<(Vec<Entity>, usize, usize)> {
        let target = self.query.get(target)?;
        let adjacents = self.query.iter().filter(|ent| {
            // keep only the adjacent ones
            target.cell.x - 1 <= ent.cell.x
                && ent.cell.x <= target.cell.x + 1
                && target.cell.y - 1 <= ent.cell.y
                && ent.cell.y <= target.cell.y + 1
                && !(ent.cell.x == target.cell.x && ent.cell.y == target.cell.y)
        });
        let cnt_bombs = adjacents.clone().filter(|ent| ent.cell.is_bomb).count();
        let cnt_flagged = adjacents
            .clone()
            .filter(|ent| ent.flagged.is_some())
            .count();
        let adjacents: Vec<_> = adjacents
            // filter out flagged or uncovered cells
            .filter(|ent| ent.flagged.is_none() && ent.covered.is_some())
            .map(|ent| ent.entity)
            .collect();
        Ok((adjacents, cnt_bombs, cnt_flagged))
    }

    fn toggle_flag(&mut self, target: Entity) {
        let Ok(mut ent) = self.query.get_mut(target) else {
            return;
        };
        if ent.covered.is_none() {
            // uncovered cell can no longer be toggled
            return;
        }
        let mut entity = self.command.entity(target);
        match ent.flagged.is_some() {
            true => {
                ent.material.0 = self.materials.covered.clone();
                entity.remove::<Flagged>();
            }
            false => {
                ent.material.0 = self.materials.flagged.clone();
                entity.insert(Flagged);
            }
        }
    }

    fn uncover(&mut self, target: Entity) {
        let Ok((adjacents, cnt_bombs, cnt_flagged)) = self.count_adjacents(target) else {
            return;
        };
        let Ok(ent) = self.query.get(target) else {
            return;
        };
        if ent.flagged.is_some() {
            // don't touch the flagged cells
            return;
        }
        match ent.covered.is_some() {
            true => {
                // uncover a covered cell, we don't care if target is bomb here
                //  instead we check in on_uncover system
                self.command.entity(target).remove::<Covered>();
            }
            false => {
                if cnt_flagged >= cnt_bombs {
                    // the cell has already been uncovered, but player have flagged enough
                    // adjacent cells to uncover the remainings
                    for ent in adjacents {
                        self.command.entity(ent).remove::<Covered>();
                    }
                }
            }
        }
    }

    fn on_uncover(&mut self, target: Entity) -> bool {
        let Ok((adjacents, cnt_bombs, _)) = self.count_adjacents(target) else {
            return false;
        };

        let Ok(mut ent) = self.query.get_mut(target) else {
            return false;
        };
        if ent.cell.is_bomb {
            // uncover a bomb, Game Over
            ent.material.0 = self.materials.bomb.clone();
            return true;
        }
        // change the material depending on bomb count
        ent.material.0 = self.materials.count[cnt_bombs].clone();
        if cnt_bombs == 0 {
            // if there are no bomb in adjacent cells, recursively uncover them
            for ent in adjacents {
                self.command.entity(ent).remove::<Covered>();
            }
        }
        false
    }
}

fn setup(mut command: Commands, mut state: ResMut<NextState<GameState>>) {
    command.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            hdr: true,
            ..Default::default()
        },
        Msaa::Off,
        Bloom::NATURAL,
    ));

    state.set(GameState::Running);
}

fn success(query: Query<&Cell, With<Covered>>, mut state: ResMut<NextState<GameState>>) {
    let count = query.iter().count();
    let success = query.iter().all(|grid| grid.is_bomb);
    if count > 0 && success {
        // sometimes this system may query no cell at all, so we check if count is correct
        // if all covered cells are bombs, then the player have won
        // I don't care enough to separate win & lose
        state.set(GameState::Over);
    }
}

fn restart(mut state: ResMut<NextState<GameState>>) {
    state.set(GameState::Prepare);
}

fn prepare(
    mut command: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Materials>,
    mut state: ResMut<NextState<GameState>>,
) {
    let mesh = meshes.add(Rectangle::from_length(UNIT));
    let material = materials.covered.clone();
    // generate a new board
    let board = Board::new(X, Y, BOMBS);
    board.iter().for_each(|grid| {
        let x = (grid.x - board.columns / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
        let y = (grid.y - board.rows / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
        command
            .spawn((
                #[cfg(feature = "debug")]
                Name::new("Cell"),
                grid,
                Covered,
                Transform::from_xyz(x, y, 0.0),
                Visibility::Visible,
                Mesh2d(mesh.clone()),
                MeshMaterial2d(material.clone()),
                Pickable::default(),
            ))
            .observe(hovered)
            .observe(unhover)
            .observe(interact)
            .observe(on_uncover);
    });

    // set next state as running, is there order any problem?
    state.set(GameState::Running);
}

fn cleanup(mut command: Commands, query: Query<Entity, With<Cell>>) {
    // despawn any existing cells
    // this WILL trigger Trigger<OnRemove, Covered>
    for entity in &query {
        command.entity(entity).despawn();
    }
}

fn hovered(
    over: Trigger<Pointer<Over>>,
    // we only activate hover effects on covered cells
    mut query: Query<&mut MeshMaterial2d<ColorMaterial>, (With<Covered>, Without<Flagged>)>,
    materials: Res<Materials>,
    state: Res<State<GameState>>,
) {
    if !state.is_running() {
        // we only handle hover event when the game is `running`
        return;
    }
    if let Ok(mut material) = query.get_mut(over.target()) {
        material.0 = materials.hovered.clone();
    }
}

fn unhover(
    out: Trigger<Pointer<Out>>,
    // we only activate hover effects on covered cells
    mut query: Query<&mut MeshMaterial2d<ColorMaterial>, (With<Covered>, Without<Flagged>)>,
    materials: Res<Materials>,
    state: Res<State<GameState>>,
) {
    if !state.is_running() {
        // we only handle hover event when the game is `running`
        return;
    }
    if let Ok(mut material) = query.get_mut(out.target()) {
        material.0 = materials.covered.clone();
    }
}

fn interact(
    click: Trigger<Pointer<Click>>,
    mut interation: InterationParam,
    state: Res<State<GameState>>,
) {
    if !state.is_running() {
        // disable uncover or flag when not running
        return;
    }
    let target = click.target();
    match click.button {
        // left button means uncover
        PointerButton::Primary => {
            interation.uncover(target);
        }
        // right button means toggle flag
        PointerButton::Secondary => {
            interation.toggle_flag(target);
        }
        _ => {}
    }
}

fn on_uncover(
    trigger: Trigger<OnRemove, Covered>,
    mut interation: InterationParam,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
) {
    let target = trigger.target();
    if interation.on_uncover(target) && state.is_running() {
        // only set if we are not already GameState::Over
        next.set(GameState::Over);
    }
}

fn reveal_bombs(mut command: Commands, query: Query<(Entity, &Cell), With<Covered>>) {
    for (entity, cell) in &query {
        if cell.is_bomb {
            command.entity(entity).remove::<Covered>();
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MineSweeper;

impl Plugin for MineSweeper {
    fn build(&self, app: &mut App) {
        app.add_plugins(MeshPickingPlugin)
            .init_resource::<Materials>()
            .init_state::<GameState>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, success.run_if(in_state(GameState::Running)))
            .add_systems(OnEnter(GameState::Prepare), prepare)
            .add_systems(
                Update,
                restart
                    .run_if(in_state(GameState::Over))
                    .run_if(input_just_released(KeyCode::Space)),
            )
            .add_systems(OnEnter(GameState::Over), reveal_bombs)
            .add_systems(OnExit(GameState::Over), cleanup);
    }
}
