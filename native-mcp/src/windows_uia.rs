//! Windows UI Automation adapter. Only simple, bounded observations are sent
//! to the model. A token is never an OS handle and must be re-resolved.
use crate::ui_description::{Bounds, UiElement};
use uiautomation::{UIAutomation, UIElement, UITreeWalker};
use uiautomation::types::{ControlType, Handle};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[derive(Debug, Clone)]
pub struct Target {
    pub runtime_id: Vec<i32>,
    pub process_id: u32,
    pub name: String,
    pub role: String,
    pub bounds: Bounds,
    pub sensitive: bool,
}

#[derive(Debug)]
pub struct Observation {
    pub foreground: isize,
    pub window_title: String,
    pub elements: Vec<UiElement>,
    pub targets: Vec<Target>,
    pub truncated: bool,
}

fn hwnd() -> Result<isize, String> {
    let handle = unsafe { GetForegroundWindow() };
    if handle.0.is_null() { return Err("UNAVAILABLE: no foreground window".into()); }
    Ok(handle.0 as isize)
}

pub fn verify_foreground(expected: isize) -> Result<(), String> {
    if hwnd()? != expected { Err("STALE: foreground window changed; observe again".into()) }
    else { Ok(()) }
}

fn automation_window(automation: &UIAutomation, window: isize) -> Result<UIElement, String> {
    automation.element_from_handle(Handle::from(window)).map_err(|e| format!("UNAVAILABLE: window UI Automation: {e}"))
}

fn element_bounds(el: &UIElement) -> Option<Bounds> {
    let r = el.get_bounding_rectangle().ok()?;
    let (width, height) = (r.get_width(), r.get_height());
    if width <= 0 || height <= 0 { return None; }
    Some(Bounds {
        x: r.get_left(), y: r.get_top(),
        width: u32::try_from(width).ok()?,
        height: u32::try_from(height).ok()?,
    })
}

fn role(el: &UIElement) -> String {
    match el.get_control_type() {
        Ok(ControlType::Button) => "button",
        Ok(ControlType::Edit) => "textbox",
        Ok(ControlType::CheckBox) => "checkbox",
        Ok(ControlType::RadioButton) => "radio",
        Ok(ControlType::ComboBox) => "combobox",
        Ok(ControlType::ListItem) => "listitem",
        Ok(ControlType::MenuItem) => "menuitem",
        Ok(ControlType::TabItem) => "tab",
        Ok(ControlType::Hyperlink) => "link",
        Ok(ControlType::TreeItem) => "treeitem",
        Ok(ControlType::Slider) => "slider",
        Ok(ControlType::Text) => "text",
        Ok(ControlType::Window) => "window",
        _ => "other",
    }.into()
}

fn read_target(el: &UIElement) -> Option<Target> {
    if el.is_offscreen().unwrap_or(true) || !el.is_enabled().unwrap_or(false) { return None; }
    let bounds = element_bounds(el)?;
    let runtime_id = el.get_runtime_id().ok()?;
    if runtime_id.is_empty() { return None; }
    let process_id = el.get_process_id().ok()?;
    let name = el.get_name().unwrap_or_default().chars().take(160).collect();
    let role = role(el);
    let sensitive = el.is_password().unwrap_or(true); // inaccessible security property fails closed
    Some(Target { runtime_id, process_id, name, role, bounds, sensitive })
}

fn crawl(walker: &UITreeWalker, root: &UIElement, max_nodes: usize)
    -> (Vec<(Target, UIElement)>, bool)
{
    // Cap visited nodes as well as emitted controls: pathological providers
    // must not produce unbounded recursion or huge serialized observations.
    let mut stack = vec![(root.clone(), 0usize)];
    let mut visited = 0;
    let mut found = Vec::new();
    while let Some((el, depth)) = stack.pop() {
        visited += 1;
        if visited > max_nodes * 8 { return (found, true); }
        if let Some(target) = read_target(&el) {
            if target.role != "other" && target.role != "window" && target.role != "text" {
                found.push((target, el.clone()));
                if found.len() >= max_nodes { return (found, true); }
            }
        }
        if depth < 6 {
            if let Some(children) = walker.get_children(&el) {
                for child in children.into_iter().rev().take(200) {
                    stack.push((child, depth + 1));
                }
            }
        }
    }
    (found, false)
}

pub fn observe(snapshot_id: &str, max_elements: usize) -> Result<Observation, String> {
    let foreground = hwnd()?;
    let automation = UIAutomation::new().map_err(|e| format!("UNAVAILABLE: UI Automation: {e}"))?;
    let root = automation_window(&automation, foreground)?;
    let window_title = root.get_name().unwrap_or_default().chars().take(160).collect();
    let walker = automation.get_control_view_walker().map_err(|e| format!("UNAVAILABLE: control walker: {e}"))?;
    let (nodes, truncated) = crawl(&walker, &root, max_elements);
    let mut elements = Vec::with_capacity(nodes.len());
    let mut targets = Vec::with_capacity(nodes.len());
    for (target, _) in nodes {
        let index = targets.len();
        elements.push(UiElement {
            element_token: format!("e{snapshot_id}:{index}"),
            role: target.role.clone(),
            name: Some(target.name.clone()),
            value: None,
            state: vec!["enabled".into(), "visible".into()],
            actions: if target.sensitive { vec![] } else { vec!["focus".into(), "click".into()] },
            bounds: Some(target.bounds.clone()),
            sensitive: target.sensitive,
        });
        targets.push(target);
    }
    Ok(Observation { foreground, window_title, elements, targets, truncated })
}

/// Resolve by runtime ID again *on the foreground window*; never trust old
/// coordinates alone. No element object is retained across COM threads.
pub fn act_on_element(expected_window: isize, target: &Target, action: &str) -> Result<(), String> {
    if target.sensitive { return Err("ACCESS_DENIED: sensitive control".into()); }
    if hwnd()? != expected_window { return Err("STALE: foreground window changed".into()); }
    let automation = UIAutomation::new().map_err(|e| format!("UNAVAILABLE: UI Automation: {e}"))?;
    let root = automation_window(&automation, expected_window)?;
    if root.get_process_id().map_err(|e| e.to_string())? != target.process_id {
        return Err("STALE: window process changed".into());
    }
    let walker = automation.get_control_view_walker().map_err(|e| e.to_string())?;
    let (nodes, _) = crawl(&walker, &root, 200);
    let (_, live) = nodes.into_iter()
        .find(|(entry, _)| entry.runtime_id == target.runtime_id)
        .ok_or_else(|| "STALE: element no longer exists or is outside observation budget".to_string())?;
    let fresh = read_target(&live).ok_or_else(|| "NOT_VISIBLE: element unavailable".to_string())?;
    if fresh.name != target.name || fresh.role != target.role || fresh.bounds.x != target.bounds.x
        || fresh.bounds.y != target.bounds.y || fresh.bounds.width != target.bounds.width
        || fresh.bounds.height != target.bounds.height
    {
        return Err("STALE: element identity or bounds changed; observe again".into());
    }
    if fresh.sensitive { return Err("ACCESS_DENIED: sensitive control".into()); }
    if hwnd()? != expected_window { return Err("STALE: foreground window changed during resolution".into()); }
    match action {
        "focus" => live.set_focus().map_err(|e| format!("ACCESS_DENIED: focus failed: {e}")),
        "click" => live.click().map_err(|e| format!("PARTIAL_EXECUTION: click delivery uncertain: {e}")),
        _ => Err("INVALID_ARGUMENT: supported actions are focus and click".into()),
    }
}

#[derive(Debug)]
pub struct ConditionResult {
    pub matched: usize,
    pub scanned: usize,
    pub truncated: bool,
}

/// Live UIA name/role query; an absent result is inconclusive when the
/// provider tree is truncated or empty. Never interpret absence as task success
/// without adequate observation coverage.
pub fn check_condition(name: &str, expected_role: Option<&str>) -> Result<ConditionResult, String> {
    let foreground = hwnd()?;
    let automation = UIAutomation::new().map_err(|e| format!("UNAVAILABLE: UI Automation: {e}"))?;
    let root = automation_window(&automation, foreground)?;
    let walker = automation.get_control_view_walker().map_err(|e| e.to_string())?;
    let (nodes, truncated) = crawl(&walker, &root, 120);
    let scanned = nodes.len();
    let matched = nodes.iter().filter(|(target, _)|
        target.name == name && expected_role.is_none_or(|role| role == target.role)
    ).count();
    Ok(ConditionResult { matched, scanned, truncated })
}
