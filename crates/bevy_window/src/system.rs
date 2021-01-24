use crate::WindowCloseRequested;
use bevy_app::{
    prelude::{EventReader, EventWriter},
    AppExit,
};

pub fn exit_on_window_close_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    if window_close_requested_events.iter().next().is_some() {
        app_exit_events.send(AppExit);
    }
}
