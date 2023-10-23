use ambient_api::{
    core::{
        app::components::window_logical_size,
        camera::{
            components::*,
            concepts::{OrthographicCamera, OrthographicCameraOptional},
        },
        messages::Frame,
    },
    prelude::*,
};

mod constants;
use constants::*;
use packages::this::{messages::Input, types::Direction};

#[main]
pub fn main() {
    let camera_id = OrthographicCamera {
        optional: OrthographicCameraOptional {
            main_scene: Some(()),
            ..default()
        },
        ..OrthographicCamera::suggested()
    }
    .spawn();

    // Update camera so we have correct aspect ratio
    change_query(window_logical_size())
        .track_change(window_logical_size())
        .bind(move |windows| {
            for (_, window) in windows {
                let window = window.as_vec2();
                if window.x <= 0. || window.y <= 0. {
                    continue;
                }

                let x_boundary = X_BOUNDARY + SCREEN_PADDING;
                let y_boundary = Y_BOUNDARY + SCREEN_PADDING;
                let (left, right, top, bottom) = if window.x < window.y {
                    (
                        -x_boundary,
                        x_boundary,
                        y_boundary * window.y / window.x,
                        -y_boundary * window.y / window.x,
                    )
                } else {
                    (
                        -x_boundary * window.x / window.y,
                        x_boundary * window.x / window.y,
                        y_boundary,
                        -y_boundary,
                    )
                };
                entity::set_component(camera_id, orthographic_left(), left);
                entity::set_component(camera_id, orthographic_right(), right);
                entity::set_component(camera_id, orthographic_top(), top);
                entity::set_component(camera_id, orthographic_bottom(), bottom);
            }
        });

    Frame::subscribe(|_| {
        let (delta, _) = input::get_delta();

        let direction = if delta.keys.contains(&KeyCode::Up) || delta.keys.contains(&KeyCode::W) {
            Some(Direction::Up)
        } else if delta.keys.contains(&KeyCode::Down) || delta.keys.contains(&KeyCode::S) {
            Some(Direction::Down)
        } else if delta.keys.contains(&KeyCode::Left) || delta.keys.contains(&KeyCode::A) {
            Some(Direction::Left)
        } else if delta.keys.contains(&KeyCode::Right) || delta.keys.contains(&KeyCode::D) {
            Some(Direction::Right)
        } else {
            None
        };

        if let Some(direction) = direction {
            Input::new(direction).send_server_reliable();
        }
    });
}
