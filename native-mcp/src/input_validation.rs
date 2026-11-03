//! Side-effect-free validation for desktop input batches.
//!
//! Every action in a batch must pass validation before Enigo is created or
//! any input is delivered. Monitor coordinates use global physical pixels.
use std::collections::HashSet;

use enigo::{Button, Key};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// Move cursor using global desktop coordinates.
    Move { x: i32, y: i32 },
    /// Click a point; button: left, right, or middle.
    Click { x: i32, y: i32, button: Option<String> },
    /// Enter text into the focused control (never stored in logs).
    Text { text: String },
    /// Press key: enter, tab, escape, backspace, delete, arrows, or a character.
    Key { key: String },
    /// Modifier(s) followed by the key to press, e.g. ["ctrl", "a"].
    Hotkey { keys: Vec<String> },
    /// Vertical scroll (positive up, negative down).
    Scroll { amount: i32 },
}

pub enum PreparedAction {
    Move { x: i32, y: i32 },
    Click { x: i32, y: i32, button: Button },
    Text { text: String },
    Key { key: Key },
    Hotkey { keys: Vec<Key> },
    Scroll { amount: i32 },
}

#[derive(Clone, Copy, Debug)]
pub struct DisplayRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl DisplayRect {
    fn contains(&self, x: i32, y: i32) -> bool {
        // i64 math prevents overflow with large or negative monitor origins.
        let (x, y) = (i64::from(x), i64::from(y));
        let (left, top) = (i64::from(self.x), i64::from(self.y));
        self.width > 0
            && self.height > 0
            && x >= left
            && x < left + i64::from(self.width)
            && y >= top
            && y < top + i64::from(self.height)
    }
}

pub fn needs_pointer(actions: &[Action]) -> bool {
    actions.iter().any(|action| matches!(action, Action::Move { .. } | Action::Click { .. }))
}

fn point_on_display(displays: &[DisplayRect], step: usize, x: i32, y: i32) -> Result<(), String> {
    if displays.iter().any(|display| display.contains(x, y)) {
        Ok(())
    } else {
        Err(format!("step {step}: global point ({x},{y}) is outside attached display bounds"))
    }
}

fn modifier_key(name: &str) -> Option<(&'static str, Key)> {
    match name.to_ascii_lowercase().as_str() {
        "ctrl" | "control" => Some(("ctrl", Key::Control)),
        "alt" => Some(("alt", Key::Alt)),
        "shift" => Some(("shift", Key::Shift)),
        "meta" | "super" | "win" | "command" => Some(("meta", Key::Meta)),
        _ => None,
    }
}

fn parse_key(name: &str) -> Result<Key, String> {
    if let Some((_, key)) = modifier_key(name) {
        return Ok(key);
    }
    let lowered = name.to_ascii_lowercase();
    let key = match lowered.as_str() {
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        other => {
            let mut chars = other.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Key::Unicode(c),
                _ => return Err(format!("unsupported key: {name}")),
            }
        }
    };
    Ok(key)
}

/// Validates the ENTIRE batch without any OS input. A later invalid step
/// cannot leave the earlier valid steps partially executed.
pub fn prepare_actions(actions: Vec<Action>, displays: &[DisplayRect]) -> Result<Vec<PreparedAction>, String> {
    if !(1..=32).contains(&actions.len()) {
        return Err("actions must contain 1..32 items".into());
    }
    let mut prepared = Vec::with_capacity(actions.len());
    for (step, action) in actions.into_iter().enumerate() {
        let item = match action {
            Action::Move { x, y } => {
                point_on_display(displays, step, x, y)?;
                PreparedAction::Move { x, y }
            }
            Action::Click { x, y, button } => {
                point_on_display(displays, step, x, y)?;
                let button = match button.as_deref().unwrap_or("left").to_ascii_lowercase().as_str() {
                    "left" => Button::Left,
                    "right" => Button::Right,
                    "middle" => Button::Middle,
                    other => return Err(format!("step {step}: unknown mouse button: {other}")),
                };
                PreparedAction::Click { x, y, button }
            }
            Action::Text { text } => {
                if text.len() > 16_384 {
                    return Err(format!("step {step}: text is too long (maximum 16384 UTF-8 bytes)"));
                }
                PreparedAction::Text { text }
            }
            Action::Key { key } => PreparedAction::Key {
                key: parse_key(&key).map_err(|error| format!("step {step}: {error}"))?,
            },
            Action::Hotkey { keys } => {
                if !(2..=5).contains(&keys.len()) {
                    return Err(format!("step {step}: hotkey needs 1..4 modifiers and one key"));
                }
                let mut seen = HashSet::new();
                let mut parsed = Vec::with_capacity(keys.len());
                for name in &keys[..keys.len() - 1] {
                    let (canonical, key) = modifier_key(name)
                        .ok_or_else(|| format!("step {step}: hotkey prefix must contain only modifiers: {name}"))?;
                    if !seen.insert(canonical) {
                        return Err(format!("step {step}: duplicate hotkey modifier: {name}"));
                    }
                    parsed.push(key);
                }
                let final_name = &keys[keys.len() - 1];
                if modifier_key(final_name).is_some() {
                    return Err(format!("step {step}: hotkey must end with a non-modifier key"));
                }
                parsed.push(parse_key(final_name).map_err(|error| format!("step {step}: {error}"))?);
                PreparedAction::Hotkey { keys: parsed }
            }
            Action::Scroll { amount } => {
                if amount.unsigned_abs() > 30 {
                    return Err(format!("step {step}: scroll limit is 30"));
                }
                PreparedAction::Scroll { amount }
            }
        };
        prepared.push(item);
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn displays() -> [DisplayRect; 2] {
        [
            DisplayRect { x: -1920, y: -100, width: 1920, height: 1080 },
            DisplayRect { x: 0, y: 0, width: 1920, height: 1080 },
        ]
    }

    #[test]
    fn supports_secondary_display_negative_coordinates() {
        let actions = vec![
            Action::Move { x: -1920, y: -100 },
            Action::Click { x: -1, y: 979, button: None },
            Action::Move { x: 1919, y: 1079 },
        ];
        assert_eq!(prepare_actions(actions, &displays()).unwrap().len(), 3);
    }

    #[test]
    fn rejects_offscreen_and_exclusive_right_bottom_edges() {
        for (x, y) in [(-1921, 0), (1920, 1), (0, 1080), (-1, 980), (10, -1)] {
            let err = prepare_actions(vec![Action::Click { x, y, button: None }], &displays()).err().unwrap();
            assert!(err.contains("outside attached display"), "{err}");
        }
    }

    #[test]
    fn rejects_monitor_gaps_and_zero_sized_monitors() {
        let displays = [
            DisplayRect { x: -100, y: 0, width: 50, height: 100 },
            DisplayRect { x: 10, y: 0, width: 0, height: 100 },
        ];
        let err = prepare_actions(vec![Action::Move { x: -25, y: 10 }], &displays).err().unwrap();
        assert!(err.contains("outside attached display"));
    }

    #[test]
    fn detects_invalid_last_step_before_any_execution() {
        let actions = vec![
            Action::Text { text: "safe-first-step".into() },
            Action::Hotkey { keys: vec!["a".into(), "b".into()] },
        ];
        assert!(prepare_actions(actions, &[]).err().unwrap().contains("step 1"));
    }

    #[test]
    fn validates_hotkeys_and_normalizes_modifier_aliases() {
        let good = Action::Hotkey { keys: vec!["Control".into(), "SHIFT".into(), "a".into()] };
        assert_eq!(prepare_actions(vec![good], &[]).unwrap().len(), 1);
        let invalid = [
            vec!["ctrl", "control", "a"],
            vec!["a", "b"],
            vec!["ctrl", "shift"],
            vec!["ctrl"],
            vec!["ctrl", "not-a-key"],
        ];
        for keys in invalid {
            let action = Action::Hotkey { keys: keys.into_iter().map(String::from).collect() };
            assert!(prepare_actions(vec![action], &[]).is_err());
        }
    }

    #[test]
    fn validates_button_keys_scroll_and_text_size() {
        let invalid = [
            Action::Click { x: 10, y: 10, button: Some("side".into()) },
            Action::Key { key: "not-a-key".into() },
            Action::Scroll { amount: i32::MIN },
            Action::Text { text: "x".repeat(16_385) },
        ];
        for action in invalid {
            assert!(prepare_actions(vec![action], &displays()).is_err());
        }
    }

    #[test]
    fn enforces_batch_size_and_non_pointer_actions_need_no_displays() {
        assert!(prepare_actions(vec![], &[]).is_err());
        let actions = (0..33).map(|_| Action::Key { key: "tab".into() }).collect();
        assert!(prepare_actions(actions, &[]).is_err());
        assert!(prepare_actions(vec![Action::Key { key: "enter".into() }], &[]).is_ok());
    }

    #[test]
    fn prevents_coordinate_overflow() {
        let extreme = [DisplayRect { x: i32::MAX, y: i32::MAX, width: u32::MAX, height: u32::MAX }];
        assert!(prepare_actions(vec![Action::Move { x: i32::MAX, y: i32::MAX }], &extreme).is_ok());
        assert!(prepare_actions(vec![Action::Move { x: i32::MIN, y: i32::MAX }], &extreme).is_err());
    }

    #[test]
    fn only_pointer_actions_require_monitor_enumeration() {
        assert!(!needs_pointer(&[Action::Key { key: "enter".into() }]));
        assert!(needs_pointer(&[Action::Scroll { amount: 1 }, Action::Move { x: 1, y: 1 }]));
    }
}
