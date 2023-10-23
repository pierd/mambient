use std::sync::Arc;

use ambient_api::{
    core::{
        messages::Frame,
        player::components::is_player,
        primitives::{
            components::{cube, sphere, sphere_radius, sphere_sectors, sphere_stacks},
            concepts::Sphere,
        },
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

fn starting_snake(head_id: EntityId) -> Entity {
    Entity::new()
        .with(frames_until_move(), STARTING_FRAMES_PER_SQUARE)
        .with(frames_per_square(), STARTING_FRAMES_PER_SQUARE)
        .with(snake_direction(), Direction::Right)
        .with(snake_current_length(), 1)
        .with(snake_target_length(), STARTING_LENGTH)
        .with(snake_head(), head_id)
        .with(snake_tail(), head_id)
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
                    Entity::new()
                        .with(scale(), vec3(x_size, y_size, 1.))
                        .with(
                            translation(),
                            vec3(
                                -X_BOUNDARY + x_size * x as f32,
                                -Y_BOUNDARY + y_size * y as f32,
                                0.,
                            ),
                        )
                        .with(
                            state(),
                            if wall {
                                SquareState::Wall
                            } else {
                                SquareState::Empty
                            },
                        )
                        .with(cube(), ())
                        .with(
                            color(),
                            if wall {
                                vec4(255., 255., 255., 1.)
                            } else {
                                vec4(0., 0., 0., 1.)
                            },
                        )
                        .with(grid_coords(), uvec2(x, y))
                        .spawn()
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

    change_query((grid_coords(), state(), color()))
        .track_change(state())
        .bind(|results| {
            for (entity_id, (_, state, _)) in results {
                let is_cube = entity::has_component(entity_id, cube());
                let should_be_cube = state != SquareState::Food;
                if is_cube != should_be_cube {
                    if should_be_cube {
                        entity::remove_components(
                            entity_id,
                            &[
                                &sphere(),
                                &sphere_radius(),
                                &sphere_sectors(),
                                &sphere_stacks(),
                            ],
                        );
                        entity::add_component(entity_id, cube(), ());
                    } else {
                        entity::remove_component(entity_id, cube());
                        entity::add_components(entity_id, Sphere::suggested().make());
                    }
                }
                entity::set_component(
                    entity_id,
                    color(),
                    match state {
                        SquareState::Empty => vec4(0., 0., 0., 1.),
                        SquareState::Snake => vec4(10., 10., 10., 1.),
                        SquareState::Wall => vec4(255., 255., 255., 1.),
                        SquareState::Food => vec4(120., 120., 120., 1.),
                    },
                );
            }
        });

    spawn_query(is_player()).bind({
        let grid = grid.clone();
        move |results| {
            for (player_id, _) in results {
                let position = uvec2(2, random::<u32>() % (GRID_HEIGHT - 2) + 1);
                let head_id = grid[position.x as usize][position.y as usize];
                entity::set_component(head_id, state(), SquareState::Snake);
                entity::add_components(player_id, starting_snake(head_id));
            }
        }
    });

    Input::subscribe(|ctx, msg| {
        if let Some(player_id) = ctx.client_entity_id() {
            let current = entity::get_component(player_id, snake_direction()).unwrap();
            match (current, msg.direction) {
                (Direction::Up | Direction::Down, Direction::Up | Direction::Down) => return,
                (Direction::Left | Direction::Right, Direction::Left | Direction::Right) => return,
                _ => {}
            }
            entity::set_component(player_id, snake_direction(), msg.direction);
        }
    });

    query((is_player(), frames_until_move())).each_frame({
        let grid = grid.clone();
        move |results| {
            for (player_id, (_, mut until_move)) in results {
                until_move = until_move.saturating_sub(1);
                if until_move == 0 {
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
                                entity::set_component(tail_id, state(), SquareState::Empty);
                            }
                            let tail_id = entity::get_component(player_id, snake_tail()).unwrap();
                            entity::set_component(tail_id, state(), SquareState::Empty);
                            // set up a new one
                            let position = uvec2(2, random::<u32>() % (GRID_HEIGHT - 2) + 1);
                            let head_id = grid[position.x as usize][position.y as usize];
                            entity::set_component(head_id, state(), SquareState::Snake);
                            entity::set_components(player_id, starting_snake(head_id));
                            continue;
                        }
                        SquareState::Food => {
                            // snake eats the food
                            entity::mutate_component(player_id, snake_target_length(), |l| {
                                *l += LENGTH_PER_FOOD
                            })
                            .unwrap();
                            // force food spawning
                            entity::set_component(entity::resources(), frames_until_food(), 0);
                        }
                    }
                    entity::add_component(head_id, snake_next(), new_head_id);
                    entity::set_component(new_head_id, state(), SquareState::Snake);
                    entity::set_component(player_id, snake_head(), new_head_id);
                    let mut current_len =
                        entity::mutate_component(player_id, snake_current_length(), |l| *l += 1)
                            .unwrap();

                    // move the tail
                    let target_len =
                        entity::get_component(player_id, snake_target_length()).unwrap();
                    while current_len > target_len {
                        let tail_id = entity::get_component(player_id, snake_tail()).unwrap();
                        entity::set_component(tail_id, state(), SquareState::Empty);
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
                    entity::set_component(id, state(), SquareState::Food);
                }
                until_food = FRAMES_PER_FOOD_SPAWN;
            }
            entity::set_component(entity::resources(), frames_until_food(), until_food);
        }
    });
}
