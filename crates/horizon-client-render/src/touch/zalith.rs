//! Reads a Zalith Launcher 2 touch layout (`layers` of `normalButtons`) as
//! Horizon's custom buttons. Layers 1 to 3 become Horizon's three control
//! modes (Crosshair, Tap, D-Pad); a button shows only in its own mode.
//!
//! Positions are normalized; Zalith sizes are fractions of the screen height,
//! as Horizon's are. The screen aspect is not known when a file is read, so
//! the width uses `ASSUMED_ASPECT`.
use super::{CustomButton, Mode, Placement, Settings};
use serde_json::Value;

/// The token a Zalith layer-switch button becomes: it moves to the next mode.
pub const MODE_NEXT: &str = "touch.mode.next";

/// Width over height used to place buttons horizontally (landscape phone).
const ASSUMED_ASPECT: f32 = 2.0;

/// Whether `root` is a Zalith layout rather than Horizon's own file.
pub fn is_zalith(root: &Value) -> bool {
    root.get("layers").is_some_and(Value::is_array)
}

/// The Horizon token for one Zalith click event, if Horizon has one.
pub fn event_token(event_type: &str, key: &str) -> Option<String> {
    let token = match event_type {
        "switch_layer" => return Some(MODE_NEXT.into()),
        "key" => glfw_key(key)?,
        "launcher_event" => match key {
            "GLFW_MOUSE_BUTTON_LEFT" => "key.mouse.left".into(),
            "GLFW_MOUSE_BUTTON_RIGHT" => "key.mouse.right".into(),
            "GLFW_MOUSE_BUTTON_MIDDLE" => "key.mouse.middle".into(),
            "launcher.event.scroll_up.single" => "mouse.scroll.up".into(),
            "launcher.event.scroll_down.single" => "mouse.scroll.down".into(),
            // The IME switch has no Horizon equivalent.
            _ => return None,
        },
        _ => return None,
    };
    CustomButton::valid_token(&token).then_some(token)
}

/// `GLFW_KEY_W` → `key.keyboard.w`, `GLFW_KEY_LEFT_SHIFT` → `key.keyboard.left.shift`.
fn glfw_key(key: &str) -> Option<String> {
    let name = key.strip_prefix("GLFW_KEY_")?;
    let special = match name {
        "SPACE" => "space",
        "ESCAPE" => "escape",
        "TAB" => "tab",
        "LEFT_SHIFT" => "left.shift",
        "RIGHT_SHIFT" => "right.shift",
        "LEFT_CONTROL" => "left.control",
        "RIGHT_CONTROL" => "right.control",
        _ => "",
    };
    if !special.is_empty() {
        return Some(format!("key.keyboard.{special}"));
    }
    // Letters, digits and F1 to F12 are plain lowercase names in Horizon.
    let function_key = name.strip_prefix('F').is_some_and(|n| {
        !n.is_empty() && n.len() <= 2 && n.bytes().all(|b| b.is_ascii_digit())
    });
    let plain = name.len() == 1 && name.bytes().all(|b| b.is_ascii_alphanumeric());
    (plain || function_key).then(|| format!("key.keyboard.{}", name.to_ascii_lowercase()))
}

/// Builds Horizon settings from a Zalith layout. Anything not in the file keeps
/// Horizon's defaults.
pub fn convert(root: &Value) -> Settings {
    let mut settings = Settings::default();
    settings.custom_buttons.clear();
    let layers = root.get("layers").and_then(Value::as_array);
    for (slot, layer) in layers.into_iter().flatten().take(Mode::ALL.len()).enumerate() {
        let buttons = layer.get("normalButtons").and_then(Value::as_array);
        for button in buttons.into_iter().flatten() {
            if let Some(custom) = custom_button(button, slot) {
                settings.custom_buttons.push(custom);
            }
        }
    }
    settings
}

/// One Zalith button, visible only in the mode of its layer `slot`.
fn custom_button(button: &Value, slot: usize) -> Option<CustomButton> {
    let tokens: Vec<String> = button
        .get("clickEvents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| {
            let kind = event.get("type")?.as_str()?;
            let key = event.get("key")?.as_str()?;
            event_token(kind, key)
        })
        .take(3)
        .collect();
    let key = tokens.first()?.clone();
    let mut custom = CustomButton::new(key, 0);
    custom.second_key = tokens.get(1).cloned().unwrap_or_default();
    custom.third_key = tokens.get(2).cloned().unwrap_or_default();
    custom.custom_label = button
        .pointer("/text/default")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let flag = |name: &str| button.get(name).and_then(Value::as_bool).unwrap_or(false);
    let x = number(button.pointer("/position/x"));
    let y = number(button.pointer("/position/y"));
    let size = button.get("buttonSize");
    let height = number(size.and_then(|s| s.get("heightPercentage"))) / 10_000.0;
    let width = number(size.and_then(|s| s.get("widthPercentage"))) / 10_000.0;
    let height = if height > 0.0 { height } else { 0.1 };
    let width_ratio = if width > 0.0 { width / height } else { 1.0 };
    let half_width = height * width_ratio / ASSUMED_ASPECT / 2.0;
    let center = [
        x / 10_000.0 + half_width,
        y / 10_000.0 + height / 2.0,
    ];
    for (index, layout) in custom.layouts.iter_mut().enumerate() {
        let mut placement = Placement::new(center[0], center[1], height);
        placement.width_ratio = width_ratio;
        placement.toggleable = flag("isToggleable");
        placement.swipeable = flag("isSwipple");
        placement.passthrough = flag("isPenetrable");
        placement.visible_in_game = true;
        placement.opacity = if index == slot { 0.85 } else { 0.0 };
        *layout = placement;
    }
    Some(custom)
}

fn number(value: Option<&Value>) -> f32 {
    value.and_then(Value::as_f64).unwrap_or(0.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn glfw_keys_become_vanilla_tokens() {
        assert_eq!(event_token("key", "GLFW_KEY_W").as_deref(), Some("key.keyboard.w"));
        assert_eq!(event_token("key", "GLFW_KEY_1").as_deref(), Some("key.keyboard.1"));
        assert_eq!(event_token("key", "GLFW_KEY_F3").as_deref(), Some("key.keyboard.f3"));
        assert_eq!(
            event_token("key", "GLFW_KEY_LEFT_SHIFT").as_deref(),
            Some("key.keyboard.left.shift")
        );
        assert_eq!(event_token("key", "GLFW_KEY_UNKNOWN_NAME"), None);
    }

    #[test]
    fn mouse_and_layer_events_map() {
        assert_eq!(
            event_token("launcher_event", "GLFW_MOUSE_BUTTON_LEFT").as_deref(),
            Some("key.mouse.left")
        );
        assert_eq!(event_token("switch_layer", "619ef64e376f").as_deref(), Some(MODE_NEXT));
        assert_eq!(event_token("launcher_event", "launcher.event.switch_ime"), None);
    }

    #[test]
    fn a_layer_button_shows_only_in_its_own_mode() {
        let root = json!({
            "layers": [
                {"normalButtons": [{
                    "text": {"default": "W"},
                    "position": {"x": 1000, "y": 2000},
                    "buttonSize": {"widthPercentage": 1000, "heightPercentage": 1000},
                    "clickEvents": [{"type": "key", "key": "GLFW_KEY_W"}],
                    "isSwipple": true
                }]},
                {"normalButtons": []},
                {"normalButtons": []}
            ]
        });
        assert!(is_zalith(&root));
        let settings = convert(&root);
        assert_eq!(settings.custom_buttons.len(), 1);
        let button = &settings.custom_buttons[0];
        assert_eq!(button.key, "key.keyboard.w");
        assert_eq!(button.custom_label, "W");
        assert!(button.layouts[0].opacity > 0.0);
        assert_eq!(button.layouts[1].opacity, 0.0);
        assert_eq!(button.layouts[2].opacity, 0.0);
        assert!(button.layouts[0].swipeable);
    }

    #[test]
    fn a_horizon_file_is_not_zalith() {
        assert!(!is_zalith(&json!({"custom_buttons": []})));
    }
}
