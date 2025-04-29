use bevy::{
    color::palettes::{
        css::{GREEN, RED, WHITE},
        tailwind::{RED_200, RED_400, RED_600, RED_800, RED_900, SKY_300, SKY_400},
    },
    core_pipeline::bloom::Bloom,
    prelude::*,
};
use rand::seq::SliceRandom;

const X: i32 = 30;
const Y: i32 = 16;
const COUNT: i32 = 70;
const UNIT: f32 = 48.0;
const GAP: f32 = 2.0;
const PADDING: f32 = 24.0;
pub const SCREEN_WIDTH: f32 = X as f32 * UNIT + (X - 1) as f32 * GAP + PADDING * 2.0;
pub const SCREEN_HEIGHT: f32 = Y as f32 * UNIT + (Y - 1) as f32 * GAP + PADDING * 2.0;

#[derive(Clone, Debug, Resource)]
pub struct Materials {
    covered: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    uncovered: Handle<ColorMaterial>,

    // remove these materials later
    flagged: Handle<ColorMaterial>,
    bomb: Handle<ColorMaterial>,
    count: [Handle<ColorMaterial>; 8],
}

impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let mut mats = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        let covered = mats.add(Color::from(SKY_400));
        let hovered = mats.add(Color::from(SKY_300));
        let uncovered = mats.add(Color::from(WHITE));
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
            uncovered,
            flagged,
            bomb,
            count,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum GridState {
    Flagged,
    Uncovered(i32),
    Covered,
    Bomb,
}

// TODO: change this event to observe Trigger<OnRemove, Covered>
#[derive(Clone, Copy, Debug, Event)]
pub struct Uncover {
    x: i32,
    y: i32,
    entity: Entity,
    manual: bool,
}

#[derive(Clone, Copy, Debug, Event)]
pub struct StartOver;

#[derive(Clone, Copy, Debug, Event)]
pub struct Flag(Entity);

#[derive(Clone, Copy, Debug, Component)]
#[require(Transform, Visibility)]
pub struct Grid {
    x: i32,
    y: i32,
    is_bomb: bool,
    state: GridState,
}

impl Grid {
    pub fn uncover(&self, entity: Entity, manual: bool) -> Uncover {
        Uncover {
            x: self.x,
            y: self.y,
            entity,
            manual,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    columns: i32,
    rows: i32,
    _bombs: i32,
    grids: Vec<Grid>,
}

impl Board {
    pub fn new(x: i32, y: i32, count: i32) -> Self {
        let mut rng = rand::rng();
        let mut grids: Vec<bool> = (0..(x * y)).map(|idx| idx < count).collect();
        grids.shuffle(&mut rng);
        let grids = grids
            .iter()
            .enumerate()
            .map(|(idx, &bomb)| Grid {
                x: idx as i32 % x,
                y: idx as i32 / x,
                is_bomb: bomb,
                state: GridState::Covered,
            })
            .collect();
        Self {
            columns: x,
            rows: y,
            _bombs: count,
            grids,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Grid> {
        self.grids.iter().copied()
    }
}

fn setup(mut command: Commands, mut events: EventWriter<StartOver>) {
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

    events.write(StartOver);
}

fn uncover(
    mut reader: EventReader<Uncover>,
    mut query: Query<(Entity, &mut MeshMaterial2d<ColorMaterial>, &mut Grid)>,
    materials: Res<Materials>,
    mut startover: EventWriter<StartOver>,
) -> Vec<Uncover> {
    let mut triggered = vec![];
    for uncover in reader.read() {
        let flagged = query
            .iter()
            .filter(|&(_, _, g)| {
                uncover.x - 1 <= g.x
                    && g.x <= uncover.x + 1
                    && uncover.y - 1 <= g.y
                    && g.y <= uncover.y + 1
                    && !(g.x == uncover.x && g.y == uncover.y)
                    && matches!(g.state, GridState::Flagged)
            })
            .count() as i32;
        let bombs = query
            .iter()
            .filter(|&(_, _, g)| {
                uncover.x - 1 <= g.x
                    && g.x <= uncover.x + 1
                    && uncover.y - 1 <= g.y
                    && g.y <= uncover.y + 1
                    && !(g.x == uncover.x && g.y == uncover.y)
                    && g.is_bomb
            })
            .count() as i32;
        let spread: Vec<_> = query
            .iter()
            .filter(|&(_, _, g)| {
                uncover.x - 1 <= g.x
                    && g.x <= uncover.x + 1
                    && uncover.y - 1 <= g.y
                    && g.y <= uncover.y + 1
                    && !(g.x == uncover.x && g.y == uncover.y)
            })
            .map(|(entity, _, g)| g.uncover(entity, false))
            .collect();
        if let Ok((_, mut mat, mut grid)) = query.get_mut(uncover.entity) {
            match grid.state {
                GridState::Uncovered(cnt) if uncover.manual && flagged == cnt => {
                    // This is problematic?
                    triggered.extend(spread)
                }
                GridState::Covered => match grid.is_bomb {
                    true => {
                        grid.state = GridState::Bomb;
                        mat.0 = materials.bomb.clone();
                        startover.write(StartOver);
                        // TODO: GameOver
                    }
                    false => match bombs {
                        0 => {
                            grid.state = GridState::Uncovered(0);
                            mat.0 = materials.uncovered.clone();
                            triggered.extend(spread)
                        }
                        cnt => {
                            grid.state = GridState::Uncovered(cnt);
                            mat.0 = materials.count[cnt as usize].clone();
                        }
                    },
                },
                _ => {}
            }
        }
    }
    triggered
}

fn spread(In(triggered): In<Vec<Uncover>>, mut writer: EventWriter<Uncover>) {
    for event in triggered {
        writer.write(event);
    }
}

fn success(query: Query<&Grid>, mut startover: EventWriter<StartOver>) {
    let success = query
        .iter()
        .all(|grid| grid.is_bomb || matches!(grid.state, GridState::Uncovered(_)));
    if success {
        startover.write(StartOver);
    }
}

fn flag(
    mut events: EventReader<Flag>,
    mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &mut Grid)>,
    materials: Res<Materials>,
) {
    for flag in events.read() {
        if let Ok((mut mat, mut grid)) = query.get_mut(flag.0) {
            match grid.state {
                GridState::Flagged => {
                    grid.state = GridState::Covered;
                    mat.0 = materials.covered.clone();
                }
                GridState::Covered => {
                    grid.state = GridState::Flagged;
                    mat.0 = materials.flagged.clone();
                }
                _ => {}
            }
        }
    }
}

fn startover(
    mut events: EventReader<StartOver>,
    mut command: Commands,
    query: Query<Entity, With<Grid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Res<Materials>,
) {
    for _ in events.read() {
        for entity in &query {
            command.entity(entity).despawn();
        }
        let mesh = meshes.add(Rectangle::from_length(UNIT));
        let material = materials.covered.clone();
        let board = Board::new(X, Y, COUNT);
        board.iter().for_each(|grid| {
            let x = (grid.x - board.columns / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
            let y = (grid.y - board.rows / 2) as f32 * (UNIT + GAP) + UNIT / 2.0;
            command
                .spawn((
                    grid,
                    Transform::from_xyz(x, y, 0.0),
                    Visibility::Visible,
                    Mesh2d(mesh.clone()),
                    MeshMaterial2d(material.clone()),
                    Pickable::default(),
                ))
                .observe(
                    move |over: Trigger<Pointer<Over>>,
                          mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &Grid)>,
                          materials: Res<Materials>| {
                        if let Ok((mut material, grid)) = query.get_mut(over.target()) {
                            if matches!(grid.state, GridState::Covered) {
                                material.0 = materials.hovered.clone();
                            }
                        }
                    },
                )
                .observe(
                    move |out: Trigger<Pointer<Out>>,
                          mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &Grid)>,
                          materials: Res<Materials>| {
                        if let Ok((mut material, grid)) = query.get_mut(out.target()) {
                            if matches!(grid.state, GridState::Covered) {
                                material.0 = materials.covered.clone();
                            }
                        }
                    },
                )
                .observe(
                    move |click: Trigger<Pointer<Click>>,
                          query: Query<&Grid>,
                          mut uncover: EventWriter<Uncover>,
                          mut flag: EventWriter<Flag>| {
                        let entity = click.target();
                        if !matches!(click.button, PointerButton::Middle) {
                            if let Ok(grid) = query.get(entity) {
                                match click.button {
                                    PointerButton::Primary => {
                                        uncover.write(grid.uncover(entity, true));
                                    }
                                    PointerButton::Secondary => {
                                        flag.write(Flag(entity));
                                    }
                                    PointerButton::Middle => {}
                                }
                            }
                        }
                    },
                );
        });
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MineSweeper;

impl Plugin for MineSweeper {
    fn build(&self, app: &mut App) {
        app.add_event::<Uncover>()
            .add_event::<Flag>()
            .add_event::<StartOver>()
            .add_plugins(MeshPickingPlugin)
            .init_resource::<Materials>()
            .add_systems(Startup, setup)
            .add_systems(FixedUpdate, (success, startover))
            .add_systems(Update, (uncover.pipe(spread), flag));
    }
}
