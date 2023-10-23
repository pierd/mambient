#![allow(dead_code)]

use ambient_api::prelude::{vec4, Vec4};

pub const X_BOUNDARY: f32 = 1.;
pub const Y_BOUNDARY: f32 = 1.;

pub const GRID_WIDTH: u32 = 50;
pub const GRID_HEIGHT: u32 = 50;

pub const EMPTY_COLOR: Vec4 = vec4(0., 0., 0., 1.);
pub const WALL_COLOR: Vec4 = vec4(255., 255., 255., 1.);

pub const STARTING_FRAMES_PER_SQUARE: u32 = 40;
pub const FRAMES_PER_SQUARE_PER_FOOD: f32 = 0.95;
pub const STARTING_LENGTH: u32 = 3;
pub const LENGTH_PER_FOOD: u32 = 2;

pub const FRAMES_PER_FOOD_SPAWN: u32 = 300;
pub const MAX_STALE_FOOD_PER_PLAYER: usize = 10;

pub const SCREEN_PADDING: f32 = 0.2;
