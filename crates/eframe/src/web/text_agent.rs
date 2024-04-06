//! The text agent is an `<input>` element used to trigger
//! mobile keyboard and IME input.
//!
use std::{cell::Cell, rc::Rc};

use wasm_bindgen::prelude::*;

use super::{AppRunner, WebRunner};

static AGENT_ID: &str = "egui_text_agent";

pub fn text_agent() -> web_sys::HtmlInputElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(AGENT_ID)
        .unwrap()
        .dyn_into()
        .unwrap()
}

/// Text event handler,
pub fn install_text_agent(state: &super::MainState) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().expect("document should have a body");
    if let Some(input) = document.get_element_by_id(AGENT_ID) {
        input.remove();
    }
    let input = document
        .create_element("input")?
        .dyn_into::<web_sys::HtmlInputElement>()?;
    let input = std::rc::Rc::new(input);
    input.set_id(AGENT_ID);
    let is_composing = Rc::new(Cell::new(false));
    {
        let style = input.style();
        // Transparent
        style.set_property("opacity", "0").unwrap();
        // Hide under canvas
        style.set_property("z-index", "-1").unwrap();
    }
    // Set size as small as possible, in case user may click on it.
    input.set_size(1);
    input.set_autofocus(true);
    input.set_hidden(true);

    // When IME is off
    state.add_event_listener(&input, "input", {
        let input_clone = input.clone();
        let is_composing = is_composing.clone();

        move |_event: web_sys::InputEvent, state| {
            let text = input_clone.value();
            if !text.is_empty() && !is_composing.get() {
                input_clone.set_value("");
                state.channels.send(egui::Event::Text(text));
                //runner.needs_repaint.repaint_asap();
            }
        }
    })?;

    {
        // When IME is on, handle composition event
        state.add_event_listener(&input, "compositionstart", {
            let input_clone = input.clone();
            let is_composing = is_composing.clone();

            move |_event: web_sys::CompositionEvent, state| {
                is_composing.set(true);
                input_clone.set_value("");

                state.channels.send(egui::Event::CompositionStart);
                //runner.needs_repaint.repaint_asap();
            }
        })?;

        state.add_event_listener(
            &input,
            "compositionupdate",
            move |event: web_sys::CompositionEvent, state| {
                if let Some(event) = event.data().map(egui::Event::CompositionUpdate) {
                    state.channels.send(event);
                    //runner.needs_repaint.repaint_asap();
                }
            },
        )?;

        state.add_event_listener(&input, "compositionend", {
            let input_clone = input.clone();

            move |event: web_sys::CompositionEvent, state| {
                is_composing.set(false);
                input_clone.set_value("");

                if let Some(event) = event.data().map(egui::Event::CompositionEnd) {
                    state.channels.send(event);
                    //runner.needs_repaint.repaint_asap();
                }
            }
        })?;
    }

    // When input lost focus, focus on it again.
    // It is useful when user click somewhere outside canvas.
    let input_refocus = input.clone();
    state.add_event_listener(
        &input,
        "focusout",
        move |_event: web_sys::MouseEvent, _state| {
            // Delay 10 ms, and focus again.
            let input_refocus = input_refocus.clone();
            call_after_delay(std::time::Duration::from_millis(10), move || {
                input_refocus.focus().ok();
            });
        },
    )?;

    body.append_child(&input)?;

    Ok(())
}

/// Focus or blur text agent to toggle mobile keyboard.
pub fn update_text_agent(state: &super::MainState) -> Option<()> {
    let inner = state.inner.borrow();

    use web_sys::HtmlInputElement;
    let window = web_sys::window()?;
    let document = window.document()?;
    let input: HtmlInputElement = document.get_element_by_id(AGENT_ID)?.dyn_into().unwrap();
    let canvas_style = state.canvas.style();

    if inner.mutable_text_under_cursor {
        let is_already_editing = input.hidden();
        if is_already_editing {
            input.set_hidden(false);
            input.focus().ok()?;

            // Move up canvas so that text edit is shown at ~30% of screen height.
            // Only on touch screens, when keyboard popups.
            if inner.touch_id.is_some() {
                let window_height = window.inner_height().ok()?.as_f64()? as f32;
                let current_rel = inner.touch_pos.y / window_height;

                // estimated amount of screen covered by keyboard
                let keyboard_fraction = 0.5;

                if current_rel > keyboard_fraction {
                    // below the keyboard

                    let target_rel = 0.3;

                    // Note: `delta` is negative, since we are moving the canvas UP
                    let delta = target_rel - current_rel;

                    let delta = delta.max(-keyboard_fraction); // Don't move it crazy much

                    let new_pos_percent = format!("{}%", (delta * 100.0).round());

                    canvas_style.set_property("position", "absolute").ok()?;
                    canvas_style.set_property("top", &new_pos_percent).ok()?;
                }
            }
        }

        // Blur and refocus the text agent (it gets refocused in the "focusout" event listener
        // after we blur it here), otherwise in Firefox, IME composition sometimes causes the IME
        // window to open in the wrong position
        call_after_delay(std::time::Duration::from_millis(0), move || {
            if input.blur().is_ok() {
                call_after_delay(std::time::Duration::from_millis(20), move || {
                    let _ = input.blur();
                });
            }
        });
    } else {
        // Holding the runner lock while calling input.blur() causes a panic.
        // This is most probably caused by the browser running the event handler
        // for the triggered blur event synchronously, meaning that the mutex
        // lock does not get dropped by the time another event handler is called.
        //
        // Why this didn't exist before #1290 is a mystery to me, but it exists now
        // and this apparently is the fix for it
        //
        // ¯\_(ツ)_/¯ - @DusterTheFirst

        // So since we are inside a runner lock here, we just postpone the blur/hide:

        call_after_delay(std::time::Duration::from_millis(0), move || {
            input.blur().ok();
            input.set_hidden(true);
            canvas_style.set_property("position", "absolute").ok();
            canvas_style.set_property("top", "0%").ok(); // move back to normal position
        });
    }
    Some(())
}

fn call_after_delay(delay: std::time::Duration, f: impl FnOnce() + 'static) {
    use wasm_bindgen::prelude::*;
    let window = web_sys::window().unwrap();
    let closure = Closure::once(f);
    let delay_ms = delay.as_millis() as _;
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            delay_ms,
        )
        .unwrap();
    closure.forget(); // We must forget it, or else the callback is canceled on drop
}

/// If context is running under mobile device?
fn is_mobile() -> Option<bool> {
    const MOBILE_DEVICE: [&str; 6] = ["Android", "iPhone", "iPad", "iPod", "webOS", "BlackBerry"];

    let user_agent = web_sys::window()?.navigator().user_agent().ok()?;
    let is_mobile = MOBILE_DEVICE.iter().any(|&name| user_agent.contains(name));
    Some(is_mobile)
}

// Move text agent to text cursor's position, on desktop/laptop,
// candidate window moves following text element (agent),
// so it appears that the IME candidate window moves with text cursor.
// On mobile devices, there is no need to do that.
pub fn move_text_cursor(
    ime: Option<egui::output::IMEOutput>,
    canvas: &web_sys::HtmlCanvasElement,
) -> Option<()> {
    let style = text_agent().style();
    // Note: moving agent on mobile devices will lead to unpredictable scroll.
    if is_mobile() == Some(false) {
        ime.as_ref().and_then(|ime| {
            let egui::Pos2 { x, y } = ime.cursor_rect.left_top();

            let bounding_rect = text_agent().get_bounding_client_rect();
            let y = (y + (canvas.scroll_top() + canvas.offset_top()) as f32)
                .min(canvas.client_height() as f32 - bounding_rect.height() as f32);
            let x = x + (canvas.scroll_left() + canvas.offset_left()) as f32;
            style.set_property("position", "absolute").ok()?;
            style.set_property("top", &format!("{y}px")).ok()?;
            style.set_property("left", &format!("{x}px")).ok()
        })
    } else {
        style.set_property("position", "absolute").ok()?;
        style.set_property("top", "0px").ok()?;
        style.set_property("left", "0px").ok()
    }
}
