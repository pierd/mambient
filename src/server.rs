use std::sync::Arc;

use ambient_api::{
    core::{
        hierarchy::components::{children, parent},
        messages::Frame,
        player::components::is_player,
        primitives::{components::cube, concepts::Sphere},
        rendering::components::color,
        transform::components::{scale, translation},
    },
    prelude::*,
};

mod constants;
use constants::*;
use packages::this::{
    components::*,
    messages::Input,
    types::{Direction, SquareState},
};
use palette::FromColor;

impl TryFrom<u8> for Direction {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Up),
            1 => Ok(Self::Down),
            2 => Ok(Self::Left),
            3 => Ok(Self::Right),
            _ => Err(value),
        }
    }
}

fn is_valid_turn(current: Direction, new: Direction) -> bool {
    !matches!(
        (current, new),
        (
            Direction::Up | Direction::Down,
            Direction::Up | Direction::Down
        )
    ) && !matches!(
        (current, new),
        (
            Direction::Left | Direction::Right,
            Direction::Left | Direction::Right
        )
    )
}

fn starting_snake(head_id: EntityId) -> Entity {
    Entity::new()
        .with(frames_until_move(), STARTING_FRAMES_PER_SQUARE)
        .with(frames_per_square(), STARTING_FRAMES_PER_SQUARE)
        .with(snake_direction(), Direction::Right)
        .with(snake_turns(), Vec::new())
        .with(snake_current_length(), 1)
        .with(snake_target_length(), STARTING_LENGTH)
        .with(snake_head(), head_id)
        .with(snake_tail(), head_id)
}

#[derive(Clone, Copy, Debug)]
enum FullSquareState {
    Empty,
    Snake(EntityId),
    Wall,
    Food,
}

impl From<FullSquareState> for SquareState {
    fn from(value: FullSquareState) -> Self {
        match value {
            FullSquareState::Empty => Self::Empty,
            FullSquareState::Snake(_) => Self::Snake,
            FullSquareState::Wall => Self::Wall,
            FullSquareState::Food => Self::Food,
        }
    }
}

fn set_state(grid_id: EntityId, new_state: FullSquareState) {
    entity::set_component(grid_id, state(), new_state.into());
    for id in entity::get_component(grid_id, children()).unwrap_or_default() {
        entity::despawn(id);
    }
    let child_id = entity::get_components(grid_id, &[&translation(), &scale()])
        .with(
            color(),
            match new_state {
                FullSquareState::Empty => EMPTY_COLOR,
                FullSquareState::Snake(player_id) => {
                    entity::get_component(player_id, color()).unwrap_or(vec4(128., 128., 128., 1.))
                }
                FullSquareState::Wall => WALL_COLOR,
                FullSquareState::Food => vec4(0xff as f32, 0xf7 as f32, 0., 1.),
            },
        )
        .with(parent(), grid_id)
        .spawn();
    if matches!(new_state, FullSquareState::Food) {
        entity::add_components(child_id, Sphere::suggested().make());
    } else {
        entity::add_component(child_id, cube(), ());
    }
}

#[main]
pub fn main() {
    let x_size = X_BOUNDARY * 2. / GRID_WIDTH as f32;
    let y_size = Y_BOUNDARY * 2. / GRID_HEIGHT as f32;
    let grid: Vec<Vec<_>> = (0..GRID_WIDTH)
        .map(|x| {
            (0..GRID_HEIGHT)
                .map(|y| {
                    let wall =
                        [0, GRID_WIDTH - 1].contains(&x) || [0, GRID_HEIGHT - 1].contains(&y);
                    let id = Entity::new()
                        .with(scale(), vec3(x_size, y_size, 1.))
                        .with(
                            translation(),
                            vec3(
                                -X_BOUNDARY + x_size * x as f32,
                                -Y_BOUNDARY + y_size * y as f32,
                                0.,
                            ),
                        )
                        .with(state(), SquareState::Empty)
                        .with(grid_coords(), uvec2(x, y))
                        .spawn();
                    set_state(
                        id,
                        if wall {
                            FullSquareState::Wall
                        } else {
                            FullSquareState::Empty
                        },
                    );
                    id
                })
                .collect()
        })
        .collect();
    let grid: Arc<Vec<_>> = Arc::from(grid);

    entity::add_component(
        entity::resources(),
        frames_until_food(),
        FRAMES_PER_FOOD_SPAWN,
    );

    spawn_query(is_player()).bind({
        let grid = grid.clone();
        move |results| {
            if !results.is_empty() {
                // force food spawning
                entity::set_component(entity::resources(), frames_until_food(), 0);
            }
            for (player_id, _) in results {
                let player_color = palette::Srgb::from_color(palette::Hsl::from_components((
                    360. * random::<f32>(),
                    1.,
                    0.5,
                )));
                let player_color = vec4(
                    player_color.red * 256.,
                    player_color.green * 256.,
                    player_color.blue * 256.,
                    1.,
                );
                entity::add_component(player_id, color(), player_color);

                let position = uvec2(2, random::<u32>() % (GRID_HEIGHT - 2) + 1);
                let head_id = grid[position.x as usize][position.y as usize];
                set_state(head_id, FullSquareState::Snake(player_id));
                entity::add_components(player_id, starting_snake(head_id));
            }
        }
    });

    Input::subscribe(|ctx, msg| {
        if let Some(player_id) = ctx.client_entity_id() {
            entity::mutate_component(player_id, snake_turns(), |turns| {
                turns.insert(0, msg.direction as u8)
            });
        }
    });

    query((is_player(), frames_until_move())).each_frame({
        let grid = grid.clone();
        move |results| {
            for (player_id, (_, mut until_move)) in results {
                until_move = until_move.saturating_sub(1);
                if until_move == 0 {
                    // apply a turn if there's one
                    let mut turns = entity::get_component(player_id, snake_turns()).unwrap();
                    if let Some(turn) = turns.pop() {
                        entity::set_component(player_id, snake_turns(), turns);
                        let current = entity::get_component(player_id, snake_direction()).unwrap();
                        let new_direction = Direction::try_from(turn).unwrap();
                        println!("turn: {:?} {:?}", current, new_direction);
                        if is_valid_turn(current, new_direction) {
                            entity::set_component(player_id, snake_direction(), new_direction);
                        }
                    }
                    // move the head
                    let head_id = entity::get_component(player_id, snake_head()).unwrap();
                    let position = entity::get_component(head_id, grid_coords()).unwrap();
                    let new_position =
                        match entity::get_component(player_id, snake_direction()).unwrap() {
                            Direction::Up => position + uvec2(0, 1),
                            Direction::Down => position - uvec2(0, 1),
                            Direction::Left => position - uvec2(1, 0),
                            Direction::Right => position + uvec2(1, 0),
                        };
                    let new_head_id = grid[new_position.x as usize][new_position.y as usize];
                    match entity::get_component(new_head_id, state()).unwrap() {
                        SquareState::Empty => { /* no-op */ }
                        SquareState::Snake | SquareState::Wall => {
                            // snake dies
                            // clear up the old snake
                            while entity::get_component(player_id, snake_head())
                                != entity::get_component(player_id, snake_tail())
                            {
                                let tail_id =
                                    entity::get_component(player_id, snake_tail()).unwrap();
                                if let Some(next) = entity::get_component(tail_id, snake_next()) {
                                    entity::set_component(player_id, snake_tail(), next);
                                }
                                entity::remove_component(tail_id, snake_next());
                                set_state(tail_id, FullSquareState::Empty);
                            }
                            let tail_id = entity::get_component(player_id, snake_tail()).unwrap();
                            set_state(tail_id, FullSquareState::Empty);
                            // set up a new one
                            let position = uvec2(2, random::<u32>() % (GRID_HEIGHT - 2) + 1);
                            let head_id = grid[position.x as usize][position.y as usize];
                            set_state(head_id, FullSquareState::Snake(player_id));
                            entity::set_components(player_id, starting_snake(head_id));
                            continue;
                        }
                        SquareState::Food => {
                            // snake eats the food
                            entity::mutate_component(player_id, snake_target_length(), |l| {
                                *l += LENGTH_PER_FOOD
                            })
                            .unwrap();
                            entity::mutate_component(player_id, frames_per_square(), |s| {
                                *s = (*s as f32 * FRAMES_PER_SQUARE_PER_FOOD) as u32
                            })
                            .unwrap();
                            // force food spawning
                            entity::set_component(entity::resources(), frames_until_food(), 0);
                        }
                    }
                    entity::add_component(head_id, snake_next(), new_head_id);
                    set_state(new_head_id, FullSquareState::Snake(player_id));
                    entity::set_component(player_id, snake_head(), new_head_id);
                    let mut current_len =
                        entity::mutate_component(player_id, snake_current_length(), |l| *l += 1)
                            .unwrap();

                    // move the tail
                    let target_len =
                        entity::get_component(player_id, snake_target_length()).unwrap();
                    while current_len > target_len {
                        let tail_id = entity::get_component(player_id, snake_tail()).unwrap();
                        set_state(tail_id, FullSquareState::Empty);
                        if let Some(next) = entity::get_component(tail_id, snake_next()) {
                            entity::set_component(player_id, snake_tail(), next);
                        }
                        entity::remove_component(tail_id, snake_next());
                        current_len =
                            entity::mutate_component(player_id, snake_current_length(), |l| {
                                *l -= 1
                            })
                            .unwrap();
                    }

                    // reset move counter
                    until_move = entity::get_component(player_id, frames_per_square()).unwrap();

                    println!("{:?}", entity::get_all_components(player_id));
                }
                entity::set_component(player_id, frames_until_move(), until_move);
            }
        }
    });

    Frame::subscribe({
        let grid = grid.clone();
        move |_| {
            let mut until_food = entity::get_component(entity::resources(), frames_until_food())
                .unwrap()
                .saturating_sub(1);
            if until_food == 0 {
                // spawn food
                let food_count = grid
                    .iter()
                    .flat_map(|v| v.iter())
                    .filter(|id| {
                        entity::get_component(**id, state()).unwrap_or(SquareState::Empty)
                            == SquareState::Food
                    })
                    .count();
                let player_count = query(is_player()).build().evaluate().len();
                if food_count < player_count * MAX_STALE_FOOD_PER_PLAYER {
                    let id = loop {
                        let x = random::<u32>() % GRID_WIDTH;
                        let y = random::<u32>() % GRID_HEIGHT;
                        let id = grid[x as usize][y as usize];
                        if entity::get_component(id, state()).unwrap() == SquareState::Empty {
                            break id;
                        }
                    };
                    set_state(id, FullSquareState::Food);
                }
                until_food = FRAMES_PER_FOOD_SPAWN;
            }
            entity::set_component(entity::resources(), frames_until_food(), until_food);
        }
    });
}
