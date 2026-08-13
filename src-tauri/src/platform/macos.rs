use tauri::WebviewWindow;

use crate::window::WorkArea;

pub fn work_area(window: &WebviewWindow) -> Result<WorkArea, String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or(window
            .primary_monitor()
            .map_err(|error| error.to_string())?)
        .ok_or_else(|| "No macOS monitor is available".to_string())?;
    let area = monitor.work_area();
    let pet_size = window.outer_size().map_err(|error| error.to_string())?;

    Ok(WorkArea {
        x: area.position.x,
        y: area.position.y,
        width: area.size.width,
        height: area.size.height,
        scale_factor: monitor.scale_factor(),
        pet_width: pet_size.width,
        pet_height: pet_size.height,
    })
}
