use serde::Serialize;
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewWindow,
};

use crate::platform;

const PET_WINDOW: &str = "pet-window";
const CHAT_BUBBLE_WINDOW: &str = "chat-bubble-window";
const SETTINGS_WINDOW: &str = "settings-window";
const BUBBLE_GAP: i32 = 2;
const INTERACTIVE_BUBBLE_SIZE: (f64, f64) = (320.0, 176.0);
const SPEECH_BUBBLE_SIZE: (f64, f64) = (300.0, 96.0);
const BUBBLE_OPEN_INTERACTIVE_EVENT: &str = "bubble-open-interactive";
const BUBBLE_PLACEMENT_CHANGED_EVENT: &str = "bubble-placement-changed";
const BUBBLE_SHOW_SPEECH_EVENT: &str = "bubble-show-speech";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeechBubblePayload {
    text: String,
    duration_ms: u64,
    source: String,
    placement: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub pet_width: u32,
    pub pet_height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PetPosition {
    pub x: i32,
    pub y: i32,
}

fn clamp_position(position: PetPosition, area: WorkArea, width: u32, height: u32) -> PetPosition {
    let max_x = area
        .x
        .saturating_add_unsigned(area.width.saturating_sub(width));
    let max_y = area
        .y
        .saturating_add_unsigned(area.height.saturating_sub(height));

    PetPosition {
        x: position.x.clamp(area.x, max_x),
        y: position.y.clamp(area.y, max_y),
    }
}

fn chat_bubble_position(
    area: WorkArea,
    pet_position: PetPosition,
    pet_size: (u32, u32),
    bubble_size: (u32, u32),
) -> (PetPosition, &'static str) {
    #[derive(Clone, Copy)]
    struct Candidate {
        position: PetPosition,
        placement: &'static str,
        space: i64,
        required: i64,
    }

    let area_left = i64::from(area.x);
    let area_top = i64::from(area.y);
    let area_right = area_left + i64::from(area.width);
    let area_bottom = area_top + i64::from(area.height);
    let pet_left = i64::from(pet_position.x);
    let pet_top = i64::from(pet_position.y);
    let pet_right = pet_left + i64::from(pet_size.0);
    let pet_bottom = pet_top + i64::from(pet_size.1);
    let bubble_width = i64::from(bubble_size.0);
    let bubble_height = i64::from(bubble_size.1);
    let gap = i64::from(BUBBLE_GAP);
    let centered_x = pet_left + i64::from(pet_size.0) / 2 - bubble_width / 2;
    let centered_y = pet_top + i64::from(pet_size.1) / 2 - bubble_height / 2;
    let candidates = [
        Candidate {
            position: PetPosition {
                x: centered_x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                y: (pet_top - bubble_height - gap).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                    as i32,
            },
            placement: "above",
            space: pet_top - area_top,
            required: bubble_height + gap,
        },
        Candidate {
            position: PetPosition {
                x: centered_x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                y: (pet_bottom + gap).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            },
            placement: "below",
            space: area_bottom - pet_bottom,
            required: bubble_height + gap,
        },
        Candidate {
            position: PetPosition {
                x: (pet_left - bubble_width - gap).clamp(i64::from(i32::MIN), i64::from(i32::MAX))
                    as i32,
                y: centered_y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            },
            placement: "left",
            space: pet_left - area_left,
            required: bubble_width + gap,
        },
        Candidate {
            position: PetPosition {
                x: (pet_right + gap).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
                y: centered_y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
            },
            placement: "right",
            space: area_right - pet_right,
            required: bubble_width + gap,
        },
    ];
    let fits = candidates
        .iter()
        .filter(|candidate| candidate.space >= candidate.required);
    let candidate = fits
        .max_by(|left, right| (left.space * right.required).cmp(&(right.space * left.required)))
        .or_else(|| {
            candidates.iter().max_by(|left, right| {
                (left.space * right.required).cmp(&(right.space * left.required))
            })
        })
        .expect("bubble placement always has four candidates");
    (
        clamp_position(candidate.position, area, bubble_size.0, bubble_size.1),
        candidate.placement,
    )
}

fn position_chat_bubble(
    app: &AppHandle,
    only_if_visible: bool,
    requested_size: Option<(u32, u32)>,
) -> Result<&'static str, String> {
    let pet = app
        .get_webview_window(PET_WINDOW)
        .ok_or_else(|| "Pet window is unavailable".to_string())?;
    let bubble = app
        .get_webview_window(CHAT_BUBBLE_WINDOW)
        .ok_or_else(|| "Chat bubble window is unavailable".to_string())?;
    if only_if_visible && !bubble.is_visible().map_err(|error| error.to_string())? {
        return Ok("above");
    }
    let area = platform::work_area(&pet)?;
    let pet_position = get_pet_position(pet.clone())?;
    let pet_size = pet.outer_size().map_err(|error| error.to_string())?;
    let bubble_size = match requested_size {
        Some((width, height)) => (width, height),
        None => {
            let size = bubble.outer_size().map_err(|error| error.to_string())?;
            (size.width, size.height)
        }
    };
    let (position, placement) = chat_bubble_position(
        area,
        pet_position,
        (pet_size.width, pet_size.height),
        bubble_size,
    );
    bubble
        .set_position(Position::Physical(PhysicalPosition::new(
            position.x, position.y,
        )))
        .map_err(|error| error.to_string())?;
    Ok(placement)
}

#[tauri::command]
pub fn get_work_area(window: WebviewWindow) -> Result<WorkArea, String> {
    platform::work_area(&window)
}

#[tauri::command]
pub fn get_pet_position(window: WebviewWindow) -> Result<PetPosition, String> {
    let position = window.outer_position().map_err(|error| error.to_string())?;
    Ok(PetPosition {
        x: position.x,
        y: position.y,
    })
}

#[tauri::command]
pub fn set_pet_position(window: WebviewWindow, x: i32, y: i32) -> Result<PetPosition, String> {
    let area = platform::work_area(&window)?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let position = clamp_position(PetPosition { x, y }, area, size.width, size.height);

    window
        .set_position(Position::Physical(PhysicalPosition::new(
            position.x, position.y,
        )))
        .map_err(|error| error.to_string())?;
    position_chat_bubble(window.app_handle(), true, None)?;
    Ok(position)
}

fn pet_size(size: &str) -> Result<f64, String> {
    match size {
        "small" => Ok(128.0),
        "medium" => Ok(160.0),
        "large" => Ok(200.0),
        _ => Err("Unknown pet size".to_string()),
    }
}

fn resize_and_clamp(window: &WebviewWindow, logical_size: LogicalSize<f64>) -> Result<(), String> {
    window
        .set_size(Size::Logical(logical_size))
        .map_err(|error| error.to_string())?;
    let area = platform::work_area(window)?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let current = get_pet_position(window.clone())?;
    let position = clamp_position(current, area, size.width, size.height);
    window
        .set_position(Position::Physical(PhysicalPosition::new(
            position.x, position.y,
        )))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_pet_window_size(window: WebviewWindow, pet_size_name: String) -> Result<(), String> {
    let size = pet_size(&pet_size_name)?;
    resize_and_clamp(&window, LogicalSize::new(size, size))?;
    position_chat_bubble(window.app_handle(), true, None).map(|_| ())
}

fn physical_bubble_size(
    bubble: &WebviewWindow,
    logical_size: (f64, f64),
) -> Result<(u32, u32), String> {
    let scale_factor = bubble.scale_factor().map_err(|error| error.to_string())?;
    Ok((
        (logical_size.0 * scale_factor).round().max(1.0) as u32,
        (logical_size.1 * scale_factor).round().max(1.0) as u32,
    ))
}

#[tauri::command]
pub fn open_chat_bubble(app: AppHandle) -> Result<String, String> {
    let bubble = app
        .get_webview_window(CHAT_BUBBLE_WINDOW)
        .ok_or_else(|| "Chat bubble window is unavailable".to_string())?;
    bubble
        .set_size(Size::Logical(LogicalSize::new(
            INTERACTIVE_BUBBLE_SIZE.0,
            INTERACTIVE_BUBBLE_SIZE.1,
        )))
        .map_err(|error| error.to_string())?;
    let requested_size = physical_bubble_size(&bubble, INTERACTIVE_BUBBLE_SIZE)?;
    let placement = position_chat_bubble(&app, false, Some(requested_size))?;
    bubble.show().map_err(|error| error.to_string())?;
    bubble.set_focus().map_err(|error| error.to_string())?;
    bubble
        .emit(BUBBLE_OPEN_INTERACTIVE_EVENT, ())
        .map_err(|error| error.to_string())?;
    bubble
        .emit(BUBBLE_PLACEMENT_CHANGED_EVENT, placement)
        .map_err(|error| error.to_string())?;
    Ok(placement.to_string())
}

#[tauri::command]
pub fn is_chat_bubble_visible(app: AppHandle) -> Result<bool, String> {
    let bubble = app
        .get_webview_window(CHAT_BUBBLE_WINDOW)
        .ok_or_else(|| "Chat bubble window is unavailable".to_string())?;
    bubble.is_visible().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_speech_bubble(
    app: AppHandle,
    text: String,
    duration_ms: u64,
    source: String,
) -> Result<Option<String>, String> {
    if !matches!(source.as_str(), "proactive" | "user_conversation") {
        return Err("Unknown speech bubble source".to_string());
    }
    let bubble = app
        .get_webview_window(CHAT_BUBBLE_WINDOW)
        .ok_or_else(|| "Chat bubble window is unavailable".to_string())?;
    if bubble.is_visible().map_err(|error| error.to_string())?
        && bubble.is_focused().map_err(|error| error.to_string())?
    {
        return Ok(None);
    }
    bubble
        .set_size(Size::Logical(LogicalSize::new(
            SPEECH_BUBBLE_SIZE.0,
            SPEECH_BUBBLE_SIZE.1,
        )))
        .map_err(|error| error.to_string())?;
    let requested_size = physical_bubble_size(&bubble, SPEECH_BUBBLE_SIZE)?;
    let placement = position_chat_bubble(&app, false, Some(requested_size))?;
    bubble.show().map_err(|error| error.to_string())?;
    if let Err(error) = bubble.emit(
        BUBBLE_SHOW_SPEECH_EVENT,
        SpeechBubblePayload {
            text,
            duration_ms: duration_ms.clamp(3_200, 20_000),
            source,
            placement: placement.to_string(),
        },
    ) {
        let _ = bubble.hide();
        return Err(error.to_string());
    }
    Ok(Some(placement.to_string()))
}

#[tauri::command]
pub fn sync_chat_bubble_position(app: AppHandle) -> Result<String, String> {
    position_chat_bubble(&app, true, None).map(str::to_string)
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    let settings = app
        .get_webview_window(SETTINGS_WINDOW)
        .ok_or_else(|| "Settings window is unavailable".to_string())?;
    settings.unminimize().map_err(|error| error.to_string())?;
    settings.show().map_err(|error| error.to_string())?;
    settings.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub fn hide_current_window(window: WebviewWindow) -> Result<(), String> {
    if window.label() == PET_WINDOW {
        return Err("The pet window cannot be hidden by this command".to_string());
    }
    window.hide().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_work_area_with_negative_origin() {
        let area = WorkArea {
            x: -1440,
            y: 25,
            width: 1440,
            height: 875,
            scale_factor: 2.0,
            pet_width: 320,
            pet_height: 320,
        };

        assert_eq!(
            clamp_position(PetPosition { x: -2000, y: 1000 }, area, 160, 160),
            PetPosition { x: -1440, y: 740 }
        );
    }

    #[test]
    fn places_bubble_near_pet_without_covering_it() {
        let area = WorkArea {
            x: 0,
            y: 25,
            width: 1440,
            height: 875,
            scale_factor: 2.0,
            pet_width: 160,
            pet_height: 160,
        };
        assert_eq!(
            chat_bubble_position(area, PetPosition { x: 600, y: 500 }, (160, 160), (320, 176)),
            (PetPosition { x: 520, y: 322 }, "above")
        );
        assert_eq!(
            chat_bubble_position(area, PetPosition { x: 10, y: 30 }, (160, 160), (320, 176)),
            (PetPosition { x: 0, y: 192 }, "below")
        );
        assert_eq!(
            chat_bubble_position(
                area,
                PetPosition { x: 1260, y: 380 },
                (160, 160),
                (320, 700)
            ),
            (PetPosition { x: 938, y: 110 }, "left")
        );
    }
}
