use super::*;

// ------------------------------------------------------------------------

/// Calls `request_animation_frame` to schedule repaint.
///
/// It will only paint if needed, but will always call `request_animation_frame` immediately.
fn paint_and_schedule(runner_ref: &WebRunner) -> Result<(), JsValue> {
    // Only paint and schedule if there has been no panic
    if let Some(mut runner_lock) = runner_ref.try_lock() {
        let mut width = runner_lock.painter.width;
        let mut height = runner_lock.painter.height;
        let mut pixel_ratio = runner_lock.painter.pixel_ratio;
        let mut modifiers = runner_lock.input.raw.modifiers;
        let mut should_save = false;
        let mut touch = None;

        for event in runner_lock
            .worker_options
            .channels
            .custom_event_rx
            .try_iter()
        {
            match event {
                WebRunnerCustomEvent::ScreenResize(new_width, new_height, new_pixel_ratio) => {
                    width = new_width;
                    height = new_height;
                    pixel_ratio = new_pixel_ratio;
                }

                WebRunnerCustomEvent::Modifiers(new_modifiers) => {
                    modifiers = new_modifiers;
                }

                WebRunnerCustomEvent::Save => {
                    should_save = true;
                }

                WebRunnerCustomEvent::Touch(touch_id, touch_pos) => {
                    touch = Some((touch_id, touch_pos));
                }
            }
        }

        // If a touch event has been detected, put it into the input and trigger a rerender
        if let Some((touch_id, touch_pos)) = touch {
            runner_lock.input.latest_touch_pos_id = touch_id;
            runner_lock.input.latest_touch_pos = Some(touch_pos);
            runner_lock.needs_repaint.repaint_asap();
        }

        // If the modifiers have changed, trigger a rerender
        if runner_lock.input.raw.modifiers != modifiers {
            runner_lock.input.raw.modifiers = modifiers;
            runner_lock.needs_repaint.repaint_asap();
        }

        runner_lock.input.raw.events = runner_lock
            .worker_options
            .channels
            .event_rx
            .try_iter()
            .collect();
        if !runner_lock.input.raw.events.is_empty() {
            // Render immediately if there are any pending events
            runner_lock.needs_repaint.repaint_asap();
        }

        // Save and rerender immediately if saving was requested
        if should_save {
            runner_lock.save();
            runner_lock.needs_repaint.repaint_asap();
        }

        // Resize the canvas if the screen size has changed
        if runner_lock.painter.width != width
            || runner_lock.painter.height != height
            || runner_lock.painter.pixel_ratio != pixel_ratio
        {
            runner_lock.painter.pixel_ratio = pixel_ratio;
            runner_lock.painter.width = width;
            runner_lock.painter.height = height;
            runner_lock.painter.surface_configuration.width =
                (width as f32 * pixel_ratio).round() as u32;
            runner_lock.painter.surface_configuration.height =
                (height as f32 * pixel_ratio).round() as u32;
            runner_lock
                .canvas
                .set_width(runner_lock.painter.surface_configuration.width);
            runner_lock
                .canvas
                .set_height(runner_lock.painter.surface_configuration.height);
            runner_lock.painter.needs_resize = true;
            // Also trigger a rerender immediately
            runner_lock.needs_repaint.repaint_asap();
        }

        paint_if_needed(&mut runner_lock);
        drop(runner_lock);
        request_animation_frame(runner_ref.clone())?;
    }
    Ok(())
}

fn paint_if_needed(runner: &mut AppRunner) {
    if runner.needs_repaint.needs_repaint() {
        if runner.has_outstanding_paint_data() {
            // We have already run the logic, e.g. in an on-click event,
            // so let's only present the results:
            runner.paint();

            // We schedule another repaint asap, so that we can run the actual logic
            // again, which may schedule a new repaint (if there's animations):
            runner.needs_repaint.repaint_asap();
        } else {
            // Clear the `needs_repaint` flags _before_
            // running the logic, as the logic could cause it to be set again.
            runner.needs_repaint.clear();

            // Run user code…
            runner.logic();

            // …and paint the result.
            runner.paint();
        }
    }
    runner.auto_save_if_needed();
}

pub(crate) fn request_animation_frame(runner_ref: WebRunner) -> Result<(), JsValue> {
    let worker = luminol_web::bindings::worker().unwrap();
    let closure = Closure::once(move || paint_and_schedule(&runner_ref));
    worker.request_animation_frame(closure.as_ref().unchecked_ref())?;
    closure.forget(); // We must forget it, or else the callback is canceled on drop
    Ok(())
}

// ------------------------------------------------------------------------

pub(crate) fn install_document_events(state: &MainState) -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();

    {
        // Avoid sticky modifier keys on alt-tab:
        for event_name in ["blur", "focus"] {
            let closure = move |event: web_sys::MouseEvent, state: &MainState| {
                let has_focus = event_name == "focus";

                if !has_focus {
                    // We lost focus - good idea to save
                    state.channels.send_custom(WebRunnerCustomEvent::Save);
                }

                //runner.input.on_web_page_focus_change(has_focus);
                //runner.egui_ctx().request_repaint();
                // log::debug!("{event_name:?}");

                state.channels.send_custom(WebRunnerCustomEvent::Modifiers(
                    modifiers_from_mouse_event(&event),
                ));
            };

            state.add_event_listener(&document, event_name, closure)?;
        }
    }

    state.add_event_listener(
        &document,
        "keydown",
        |event: web_sys::KeyboardEvent, state| {
            if event.is_composing() || event.key_code() == 229 {
                // https://web.archive.org/web/20200526195704/https://www.fxsitecompat.dev/en-CA/docs/2018/keydown-and-keyup-events-are-now-fired-during-ime-composition/
                return;
            }

            let modifiers = modifiers_from_event(&event);
            state
                .channels
                .send_custom(WebRunnerCustomEvent::Modifiers(modifiers));

            let key = event.key();
            let egui_key = translate_key(&key);

            if let Some(key) = egui_key {
                state.channels.send(egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false, // egui will fill this in for us!
                    modifiers,
                });
            }
            if !modifiers.ctrl
                && !modifiers.command
                && !should_ignore_key(&key)
                // When text agent is shown, it sends text event instead.
                && text_agent::text_agent().hidden()
            {
                state.channels.send(egui::Event::Text(key));
            }
            //runner.needs_repaint.repaint_asap();

            let egui_wants_keyboard = state.inner.borrow().wants_keyboard_input;

            #[allow(clippy::if_same_then_else)]
            let prevent_default = if egui_key == Some(egui::Key::Tab) {
                // Always prevent moving cursor to url bar.
                // egui wants to use tab to move to the next text field.
                true
            } else if matches!(
                egui_key,
                Some(egui::Key::P | egui::Key::S | egui::Key::O | egui::Key::F)
            ) {
                #[allow(clippy::needless_bool)]
                if modifiers.ctrl || modifiers.command || modifiers.mac_cmd {
                    true // Prevent ctrl-P opening the print dialog. Users may want to use it for a command palette.
                } else {
                    false // let normal P:s through
                }
            } else if egui_wants_keyboard {
                matches!(
                    event.key().as_str(),
                    "Backspace" // so we don't go back to previous page when deleting text
                    | "ArrowDown" | "ArrowLeft" | "ArrowRight" | "ArrowUp" // cmd-left is "back" on Mac (https://github.com/emilk/egui/issues/58)
                )
            } else {
                // We never want to prevent:
                // * F5 / cmd-R (refresh)
                // * cmd-shift-C (debug tools)
                // * cmd/ctrl-c/v/x (or we stop copy/past/cut events)
                false
            };

            // log::debug!(
            //     "On key-down {:?}, egui_wants_keyboard: {}, prevent_default: {}",
            //     event.key().as_str(),
            //     egui_wants_keyboard,
            //     prevent_default
            // );

            if prevent_default {
                event.prevent_default();
                // event.stop_propagation();
            }
        },
    )?;

    state.add_event_listener(
        &document,
        "keyup",
        |event: web_sys::KeyboardEvent, state| {
            let modifiers = modifiers_from_event(&event);
            state
                .channels
                .send_custom(WebRunnerCustomEvent::Modifiers(modifiers));
            if let Some(key) = translate_key(&event.key()) {
                state.channels.send(egui::Event::Key {
                    key,
                    pressed: false,
                    repeat: false,
                    modifiers,
                });
            }
            //runner.needs_repaint.repaint_asap();
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    state.add_event_listener(
        &document,
        "paste",
        |event: web_sys::ClipboardEvent, state| {
            if let Some(data) = event.clipboard_data() {
                if let Ok(text) = data.get_data("text") {
                    let text = text.replace("\r\n", "\n");
                    if !text.is_empty() {
                        state.channels.send(egui::Event::Paste(text));
                        //runner.needs_repaint.repaint_asap();
                    }
                    event.stop_propagation();
                    event.prevent_default();
                }
            }
        },
    )?;

    #[cfg(web_sys_unstable_apis)]
    state.add_event_listener(&document, "cut", |event: web_sys::ClipboardEvent, state| {
        state.channels.send(egui::Event::Cut);

        // In Safari we are only allowed to write to the clipboard during the
        // event callback, which is why we run the app logic here and now:
        //runner.logic();

        // Make sure we paint the output of the above logic call asap:
        //runner.needs_repaint.repaint_asap();

        event.stop_propagation();
        event.prevent_default();
    })?;

    #[cfg(web_sys_unstable_apis)]
    state.add_event_listener(
        &document,
        "copy",
        |event: web_sys::ClipboardEvent, state| {
            state.channels.send(egui::Event::Copy);

            // In Safari we are only allowed to write to the clipboard during the
            // event callback, which is why we run the app logic here and now:
            //runner.logic();

            // Make sure we paint the output of the above logic call asap:
            //runner.needs_repaint.repaint_asap();

            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    Ok(())
}

pub(crate) fn install_window_events(state: &MainState) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    /*

    // Save-on-close
    runner_ref.add_event_listener(&window, "onbeforeunload", |_: web_sys::Event, runner| {
        runner.save();
    })?;

    for event_name in &["load", "pagehide", "pageshow", "resize"] {
        runner_ref.add_event_listener(&window, event_name, |_: web_sys::Event, runner| {
            runner.needs_repaint.repaint_asap();
        })?;
    }

    runner_ref.add_event_listener(&window, "hashchange", |_: web_sys::Event, runner| {
        // `epi::Frame::info(&self)` clones `epi::IntegrationInfo`, but we need to modify the original here
        runner.frame.info.web_info.location.hash = location_hash();
    })?;

    */

    let closure = {
        let window = window.clone();
        move |_event: web_sys::Event, state: &MainState| {
            let pixel_ratio = window.device_pixel_ratio();
            let pixel_ratio = if pixel_ratio > 0. && pixel_ratio.is_finite() {
                pixel_ratio as f32
            } else {
                1.
            };
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
            let _ = state
                .canvas
                .set_attribute("width", width.to_string().as_str());
            let _ = state
                .canvas
                .set_attribute("height", height.to_string().as_str());
            state
                .channels
                .send_custom(WebRunnerCustomEvent::ScreenResize(
                    width,
                    height,
                    pixel_ratio,
                ));
        }
    };
    closure(web_sys::Event::new("")?, state);
    state.add_event_listener(&window, "resize", closure)?;

    Ok(())
}

pub(crate) fn install_color_scheme_change_event(runner_ref: &WebRunner) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    if let Some(media_query_list) = prefers_color_scheme_dark(&window)? {
        runner_ref.add_event_listener::<web_sys::MediaQueryListEvent>(
            &media_query_list,
            "change",
            |event, runner| {
                let theme = theme_from_dark_mode(event.matches());
                runner.frame.info.system_theme = Some(theme);
                runner.egui_ctx().set_visuals(theme.egui_visuals());
                runner.needs_repaint.repaint_asap();
            },
        )?;
    }

    Ok(())
}

pub(crate) fn install_canvas_events(state: &MainState) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();

    {
        let prevent_default_events = [
            // By default, right-clicks open a context menu.
            // We don't want to do that (right clicks is handled by egui):
            "contextmenu",
            // Allow users to use ctrl-p for e.g. a command palette:
            "afterprint",
        ];

        for event_name in prevent_default_events {
            let closure = move |event: web_sys::MouseEvent, _state: &_| {
                event.prevent_default();
                // event.stop_propagation();
                // log::debug!("Preventing event {event_name:?}");
            };

            state.add_event_listener(&state.canvas, event_name, closure)?;
        }
    }

    state.add_event_listener(
        &state.canvas,
        "mousedown",
        |event: web_sys::MouseEvent, state| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(&state.canvas, &event);
                let modifiers = modifiers_from_mouse_event(&event);
                state.channels.send(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                });

                // In Safari we are only allowed to write to the clipboard during the
                // event callback, which is why we run the app logic here and now:
                //runner.logic();

                // Make sure we paint the output of the above logic call asap:
                //runner.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            // Note: prevent_default breaks VSCode tab focusing, hence why we don't call it here.
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "mousemove",
        |event: web_sys::MouseEvent, state| {
            let pos = pos_from_mouse_event(&state.canvas, &event);
            state.channels.send(egui::Event::PointerMoved(pos));
            //runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "mouseup",
        |event: web_sys::MouseEvent, state| {
            if let Some(button) = button_from_mouse_event(&event) {
                let pos = pos_from_mouse_event(&state.canvas, &event);
                let modifiers = modifiers_from_mouse_event(&event);
                state.channels.send(egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: false,
                    modifiers,
                });

                // In Safari we are only allowed to write to the clipboard during the
                // event callback, which is why we run the app logic here and now:
                //runner.logic();

                // Make sure we paint the output of the above logic call asap:
                //runner.needs_repaint.repaint_asap();

                text_agent::update_text_agent(state);
            }
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "mouseleave",
        |event: web_sys::MouseEvent, state| {
            state.channels.send_custom(WebRunnerCustomEvent::Save);

            state.channels.send(egui::Event::PointerGone);
            //runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "touchstart",
        |event: web_sys::TouchEvent, state| {
            let mut inner = state.inner.borrow_mut();

            inner.touch_pos = pos_from_touch_event(&state.canvas, &event, &mut inner.touch_id);
            state
                .channels
                .send_custom(WebRunnerCustomEvent::Touch(inner.touch_id, inner.touch_pos));
            let modifiers = modifiers_from_touch_event(&event);
            state.channels.send(egui::Event::PointerButton {
                pos: inner.touch_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers,
            });

            push_touches(state, egui::TouchPhase::Start, &event);
            //runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "touchmove",
        |event: web_sys::TouchEvent, state| {
            let mut inner = state.inner.borrow_mut();

            inner.touch_pos = pos_from_touch_event(&state.canvas, &event, &mut inner.touch_id);
            state
                .channels
                .send_custom(WebRunnerCustomEvent::Touch(inner.touch_id, inner.touch_pos));
            state
                .channels
                .send(egui::Event::PointerMoved(inner.touch_pos));

            push_touches(state, egui::TouchPhase::Move, &event);
            //runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "touchend",
        |event: web_sys::TouchEvent, state| {
            let inner = state.inner.borrow();

            if inner.touch_id.is_some() {
                let modifiers = modifiers_from_touch_event(&event);
                // First release mouse to click:
                state.channels.send(egui::Event::PointerButton {
                    pos: inner.touch_pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                });
                // Then remove hover effect:
                state.channels.send(egui::Event::PointerGone);

                push_touches(state, egui::TouchPhase::End, &event);
                //runner.needs_repaint.repaint_asap();
            }
            event.stop_propagation();
            event.prevent_default();

            // Finally, focus or blur text agent to toggle mobile keyboard:
            text_agent::update_text_agent(state);
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "touchcancel",
        |event: web_sys::TouchEvent, state| {
            push_touches(state, egui::TouchPhase::Cancel, &event);
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    state.add_event_listener(
        &state.canvas,
        "wheel",
        |event: web_sys::WheelEvent, state| {
            let unit = match event.delta_mode() {
                web_sys::WheelEvent::DOM_DELTA_PIXEL => egui::MouseWheelUnit::Point,
                web_sys::WheelEvent::DOM_DELTA_LINE => egui::MouseWheelUnit::Line,
                web_sys::WheelEvent::DOM_DELTA_PAGE => egui::MouseWheelUnit::Page,
                _ => return,
            };
            // delta sign is flipped to match native (winit) convention.
            let delta = -egui::vec2(event.delta_x() as f32, event.delta_y() as f32);
            let modifiers = modifiers_from_wheel_event(&event);

            state.channels.send(egui::Event::MouseWheel {
                unit,
                delta,
                modifiers,
            });

            let scroll_multiplier = match unit {
                egui::MouseWheelUnit::Page => canvas_size_in_points(&state.canvas).y,
                egui::MouseWheelUnit::Line => {
                    #[allow(clippy::let_and_return)]
                    let points_per_scroll_line = 8.0; // Note that this is intentionally different from what we use in winit.
                    points_per_scroll_line
                }
                egui::MouseWheelUnit::Point => 1.0,
            };

            let mut delta = scroll_multiplier * delta;

            // Report a zoom event in case CTRL (on Windows or Linux) or CMD (on Mac) is pressed.
            // This if-statement is equivalent to how `Modifiers.command` is determined in
            // `modifiers_from_event()`, but we cannot directly use that fn for a [`WheelEvent`].
            if event.ctrl_key() || event.meta_key() {
                let factor = (delta.y / 200.0).exp();
                state.channels.send(egui::Event::Zoom(factor));
            } else {
                if event.shift_key() {
                    // Treat as horizontal scrolling.
                    // Note: one Mac we already get horizontal scroll events when shift is down.
                    delta = egui::vec2(delta.x + delta.y, 0.0);
                }

                state.channels.send(egui::Event::Scroll(delta));
            }

            //runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        },
    )?;

    /* Luminol's web filesystem can't read files from egui's file drag and drop system

    runner_ref.add_event_listener(&canvas, "dragover", |event: web_sys::DragEvent, runner| {
        if let Some(data_transfer) = event.data_transfer() {
            runner.input.raw.hovered_files.clear();
            for i in 0..data_transfer.items().length() {
                if let Some(item) = data_transfer.items().get(i) {
                    runner.input.raw.hovered_files.push(egui::HoveredFile {
                        mime: item.type_(),
                        ..Default::default()
                    });
                }
            }
            runner.needs_repaint.repaint_asap();
            event.stop_propagation();
            event.prevent_default();
        }
    })?;

    runner_ref.add_event_listener(&canvas, "dragleave", |event: web_sys::DragEvent, runner| {
        runner.input.raw.hovered_files.clear();
        runner.needs_repaint.repaint_asap();
        event.stop_propagation();
        event.prevent_default();
    })?;

    runner_ref.add_event_listener(&canvas, "drop", {
        let runner_ref = runner_ref.clone();

        move |event: web_sys::DragEvent, runner| {
            if let Some(data_transfer) = event.data_transfer() {
                runner.input.raw.hovered_files.clear();
                runner.needs_repaint.repaint_asap();

                if let Some(files) = data_transfer.files() {
                    for i in 0..files.length() {
                        if let Some(file) = files.get(i) {
                            let name = file.name();
                            let mime = file.type_();
                            let last_modified = std::time::UNIX_EPOCH
                                + std::time::Duration::from_millis(file.last_modified() as u64);

                            log::debug!("Loading {:?} ({} bytes)…", name, file.size());

                            let future = wasm_bindgen_futures::JsFuture::from(file.array_buffer());

                            let runner_ref = runner_ref.clone();
                            let future = async move {
                                match future.await {
                                    Ok(array_buffer) => {
                                        let bytes = js_sys::Uint8Array::new(&array_buffer).to_vec();
                                        log::debug!("Loaded {:?} ({} bytes).", name, bytes.len());

                                        if let Some(mut runner_lock) = runner_ref.try_lock() {
                                            runner_lock.input.raw.dropped_files.push(
                                                egui::DroppedFile {
                                                    name,
                                                    mime,
                                                    last_modified: Some(last_modified),
                                                    bytes: Some(bytes.into()),
                                                    ..Default::default()
                                                },
                                            );
                                            runner_lock.needs_repaint.repaint_asap();
                                        }
                                    }
                                    Err(err) => {
                                        log::error!("Failed to read file: {:?}", err);
                                    }
                                }
                            };
                            wasm_bindgen_futures::spawn_local(future);
                        }
                    }
                }
                event.stop_propagation();
                event.prevent_default();
            }
        }
    })?;

    */

    {
        // The canvas automatically resizes itself whenever a frame is drawn.
        // The resizing does not take window.devicePixelRatio into account,
        // so this mutation observer is to detect canvas resizes and correct them.
        let window = window.clone();
        let callback: Closure<dyn FnMut(_)> = Closure::new(move |mutations: js_sys::Array| {
            if PANIC_LOCK.get().is_some() {
                return;
            }
            let width = window.inner_width().unwrap().as_f64().unwrap() as u32;
            let height = window.inner_height().unwrap().as_f64().unwrap() as u32;
            mutations.for_each(&mut |mutation, _, _| {
                let mutation = mutation.unchecked_into::<web_sys::MutationRecord>();
                if mutation.type_().as_str() == "attributes" {
                    let canvas = mutation
                        .target()
                        .unwrap()
                        .unchecked_into::<web_sys::HtmlCanvasElement>();
                    if canvas.width() != width || canvas.height() != height {
                        let _ = canvas.set_attribute("width", width.to_string().as_str());
                        let _ = canvas.set_attribute("height", height.to_string().as_str());
                    }
                }
            });
        });
        let observer = web_sys::MutationObserver::new(callback.as_ref().unchecked_ref())?;
        let mut options = web_sys::MutationObserverInit::new();
        options.attributes(true);
        observer.observe_with_options(&state.canvas, &options)?;
        callback.forget();
    }

    Ok(())
}
