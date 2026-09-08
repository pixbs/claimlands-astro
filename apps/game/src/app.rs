use claimlands_renderer::{Backend, Renderer};
use claimlands_visuals::PlanetAssets;
use claimlands_world::{LevelDefinition, World};
use glam::{Quat, Vec3};
use std::{collections::BTreeMap, sync::Arc};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};

#[cfg(target_os = "android")]
type MobileApp = winit::platform::android::activity::AndroidApp;
#[cfg(not(target_os = "android"))]
type MobileApp = ();

enum UserEvent {
    Ready(u64, Result<Renderer, String>),
}
struct Game {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    proxy: EventLoopProxy<UserEvent>,
    generation: u64,
    world: World,
    seed: u64,
    frequency: u8,
    rotation: Quat,
    distance: f32,
    selected: Option<usize>,
    context: egui::Context,
    input: egui::RawInput,
    cursor: egui::Pos2,
    dragging: bool,
    drag_distance: f32,
    touches: BTreeMap<u64, egui::Pos2>,
    pinch: Option<f32>,
    backend: Backend,
    error: Option<String>,
    presented: bool,
    recovery_attempts: u8,
    modifiers: egui::Modifiers,
    ime_active: bool,
}

pub fn run(mobile: Option<MobileApp>) -> Result<(), String> {
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    #[cfg(target_os = "android")]
    {
        use winit::platform::android::EventLoopBuilderExtAndroid;
        builder.with_android_app(mobile.ok_or("missing Android activity")?);
    }
    #[cfg(not(target_os = "android"))]
    let _ = mobile;
    let event_loop = builder.build().map_err(|e| e.to_string())?;
    let seed = parameter("seed")
        .and_then(|x| x.parse().ok())
        .unwrap_or(63352);
    let frequency = parameter("frequency")
        .and_then(|x| x.parse::<u8>().ok())
        .filter(|n| (2..=12).contains(n))
        .unwrap_or(8);
    let backend = match parameter("backend").as_deref() {
        Some("webgl") => Backend::WebGl,
        Some("webgpu") => Backend::WebGpu,
        _ => Backend::Auto,
    };
    let context = egui::Context::default();
    context.set_theme(egui::Theme::Dark);
    let mut style = (*context.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = egui::Color32::from_rgba_unmultiplied(21, 19, 32, 238);
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(117, 78, 30);
    context.set_style_of(egui::Theme::Dark, style);
    #[allow(unused_mut)]
    let mut game = Game {
        window: None,
        renderer: None,
        proxy: event_loop.create_proxy(),
        generation: 0,
        world: World::generate(&LevelDefinition::new(frequency, seed))
            .map_err(|e| e.to_string())?,
        seed,
        frequency,
        rotation: Quat::from_rotation_x(0.25) * Quat::from_rotation_y(-0.35),
        distance: 3.5,
        selected: None,
        context,
        input: egui::RawInput::default(),
        cursor: egui::Pos2::ZERO,
        dragging: false,
        drag_distance: 0.,
        touches: BTreeMap::new(),
        pinch: None,
        backend,
        error: None,
        presented: false,
        recovery_attempts: 0,
        modifiers: egui::Modifiers::default(),
        ime_active: false,
    };
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(game);
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run_app(&mut game).map_err(|e| e.to_string())
}

impl Game {
    fn initialize_renderer(&mut self, window: Arc<Window>) {
        self.generation += 1;
        self.renderer = None;
        self.presented = false;
        self.error = None;
        status("error", "");
        status("ready", "false");
        // A new GPU renderer has no egui atlas. A fresh context forces a complete
        // atlas upload, retaining the application's controls, camera, and world.
        let style = self.context.style_of(egui::Theme::Dark);
        self.context = egui::Context::default();
        self.context.set_theme(egui::Theme::Dark);
        self.context.set_style_of(egui::Theme::Dark, style);
        self.input = egui::RawInput::default();
        self.cancel_pointer();
        let assets = PlanetAssets::generate(&self.world);
        let backend = self.backend;
        let generation = self.generation;
        let proxy = self.proxy.clone();
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let result = Renderer::new(window, &assets, backend).await;
            let _ = proxy.send_event(UserEvent::Ready(generation, result));
        });
        #[cfg(not(target_arch = "wasm32"))]
        {
            let result = pollster::block_on(Renderer::new(window, &assets, backend));
            let _ = proxy.send_event(UserEvent::Ready(generation, result));
        }
    }
    fn fail_graphics(&mut self, error: String) {
        self.renderer = None;
        self.presented = false;
        status("ready", "false");
        status("error", &error);
        self.error = Some(error);
    }
    fn recover_device(&mut self) {
        // Stop after two automatic attempts in one activation. An unavailable
        // adapter must not spin allocating devices or repeatedly requesting them.
        if self.recovery_attempts >= 2 {
            self.fail_graphics(
                "Graphics recovery failed. Resume or reload the game to retry.".into(),
            );
            return;
        }
        if let Some(window) = self.window.clone() {
            self.recovery_attempts += 1;
            status("device-recovery", &self.recovery_attempts.to_string());
            self.initialize_renderer(window);
        }
    }
    fn cancel_pointer(&mut self) {
        self.dragging = false;
        self.drag_distance = 0.;
        self.touches.clear();
        self.pinch = None;
        self.input.events.push(egui::Event::PointerButton {
            pos: self.cursor,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: self.modifiers,
        });
        self.input.events.push(egui::Event::PointerGone);
    }
    fn keyboard_input(&mut self, event: KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if !self.ime_active {
            if let Some(key) = gui_key(&event.logical_key) {
                self.input.events.push(egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: event.repeat,
                    modifiers: self.modifiers,
                });
            }
            if pressed
                && !self.modifiers.ctrl
                && !self.modifiers.command
                && let Some(text) = event.text
            {
                let text: String = text.chars().filter(|ch| !ch.is_control()).collect();
                if !text.is_empty() {
                    self.input.events.push(egui::Event::Text(text));
                }
            }
        }
        if !pressed
            || event.repeat
            || self.ime_active
            || self.modifiers.command
            || self.modifiers.ctrl
            || self.modifiers.alt
            || self.context.egui_wants_keyboard_input()
        {
            return;
        }
        match event.physical_key {
            PhysicalKey::Code(KeyCode::KeyN) => {
                self.seed = self.seed.wrapping_add(1);
                self.regenerate();
            }
            PhysicalKey::Code(KeyCode::Digit0) => {
                self.seed = 0;
                self.regenerate();
            }
            PhysicalKey::Code(KeyCode::Equal | KeyCode::NumpadAdd) => {
                self.frequency = (self.frequency + 1).min(12);
                self.regenerate();
            }
            PhysicalKey::Code(KeyCode::Minus | KeyCode::NumpadSubtract) => {
                self.frequency = self.frequency.saturating_sub(1).max(2);
                self.regenerate();
            }
            _ => {}
        }
    }
    fn regenerate(&mut self) {
        match World::generate(&LevelDefinition::new(self.frequency, self.seed)) {
            Ok(world) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.replace_world(&PlanetAssets::generate(&world));
                }
                self.world = world;
                self.selected = None;
                self.error = None;
                self.publish();
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    fn publish(&self) {
        status("seed", &self.world.seed().to_string());
        status("frequency", &self.world.frequency().to_string());
        status("tile-count", &self.world.tiles().len().to_string());
        status("fingerprint", &self.world.fingerprint());
        status(
            "selected-tile",
            &self.selected.map_or(String::new(), |id| id.to_string()),
        );
        status("distance", &format!("{:.3}", self.distance));
        status(
            "rotation",
            &format!(
                "{:.4},{:.4},{:.4},{:.4}",
                self.rotation.x, self.rotation.y, self.rotation.z, self.rotation.w
            ),
        );
    }
    fn pick(&mut self) {
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        let x = self.cursor.x * scale / size.width as f32 * 2. - 1.;
        let y = 1. - self.cursor.y * scale / size.height as f32 * 2.;
        let tangent = (38f32.to_radians() / 2.).tan();
        let direction = Vec3::new(
            x * tangent * size.width as f32 / size.height as f32,
            y * tangent,
            -1.,
        )
        .normalize();
        let origin = Vec3::new(0., 0., self.distance);
        let b = origin.dot(direction);
        let c = origin.length_squared() - 1.024144;
        let disc = b * b - c;
        self.selected = if disc >= 0. {
            let p = self.rotation.inverse() * (origin + direction * (-b - disc.sqrt()));
            self.world
                .tiles()
                .iter()
                .max_by(|a, b| {
                    Vec3::from_array(a.center().map(|v| v as f32))
                        .dot(p)
                        .total_cmp(&Vec3::from_array(b.center().map(|v| v as f32)).dot(p))
                })
                .map(|t| t.id().index())
        } else {
            None
        };
        self.publish();
    }
    fn move_pointer(&mut self, pos: egui::Pos2) {
        let delta = pos - self.cursor;
        self.cursor = pos;
        self.input.events.push(egui::Event::PointerMoved(pos));
        if self.dragging && self.touches.len() < 2 {
            self.drag_distance += delta.length();
            self.rotation = Quat::from_rotation_y(delta.x * 0.006)
                * Quat::from_rotation_x(delta.y * 0.006)
                * self.rotation;
            self.rotation = self.rotation.normalize();
            self.publish();
        }
    }
    fn press(&mut self, pressed: bool) {
        self.input.events.push(egui::Event::PointerButton {
            pos: self.cursor,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: self.modifiers,
        });
        if pressed {
            self.drag_distance = 0.;
            self.dragging = !self.pointer_over_controls();
        } else {
            if self.dragging && self.drag_distance < 5. {
                self.pick();
            }
            self.dragging = false;
        }
    }
    fn pointer_over_controls(&self) -> bool {
        // UI input is consumed once per frame. Mouse/touch events between frames
        // must be hit-tested at their current position rather than cached hover.
        self.context
            .layer_id_at(self.cursor)
            .is_some_and(|layer| layer.order != egui::Order::Background)
    }
    fn draw(&mut self) {
        if self.renderer.as_ref().is_some_and(Renderer::is_device_lost) {
            self.recover_device();
            return;
        }
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 || self.renderer.is_none() {
            return;
        }
        let scale = window.scale_factor() as f32;
        self.context.set_pixels_per_point(scale);
        self.input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(size.width as f32 / scale, size.height as f32 / scale),
        ));
        let mut regenerate = false;
        let output = self.context.run_ui(self.input.take(), |root_ui| {
            let ctx = root_ui.ctx();
            egui::Window::new("CLAIM LANDS")
                .resizable(false)
                .collapsible(false)
                .default_pos([16., 16.])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("A world waiting to be claimed")
                            .color(egui::Color32::from_rgb(167, 156, 181)),
                    );
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Seed");
                        ui.add(egui::DragValue::new(&mut self.seed).range(0..=u32::MAX as u64));
                    });
                    ui.add(egui::Slider::new(&mut self.frequency, 2..=12).text("World size"));
                    if ui.button("Generate planet").clicked() {
                        regenerate = true;
                    }
                    ui.label(format!("{} tiles · 12 pentagons", self.world.tiles().len()));
                    if let Some(id) = self.selected {
                        ui.label(format!(
                            "Tile {id} · {:?}",
                            self.world.tiles()[id].terrain()
                        ));
                    }
                    ui.separator();
                    ui.small("Drag to orbit · scroll / pinch to zoom");
                    ui.small("N: next seed · 0: ocean · +/-: size");
                    if let Some(error) = &self.error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                });
            egui::Area::new(egui::Id::new("revision"))
                .anchor(egui::Align2::RIGHT_BOTTOM, [-12., -10.])
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new(format!("FOUNDATION  /  {}", revision()))
                            .size(10.)
                            .color(egui::Color32::from_rgb(122, 114, 139)),
                    );
                });
        });
        if let Some(window) = &self.window {
            window.set_ime_allowed(self.context.egui_wants_keyboard_input());
        }
        if regenerate {
            self.regenerate();
        }
        let selected = self
            .selected
            .map(|id| self.world.tiles()[id].center().map(|v| v as f32));
        if let Some(renderer) = &mut self.renderer {
            match renderer.draw(
                self.rotation,
                self.distance,
                selected,
                &self.context,
                output,
            ) {
                Ok(true) => {
                    if !self.presented {
                        status("ready", "true");
                        self.presented = true;
                    }
                    status("backend", renderer.backend());
                }
                Ok(false) => {}
                Err(error) => self.fail_graphics(error),
            }
        }
    }
}

impl ApplicationHandler<UserEvent> for Game {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        self.recovery_attempts = 0;
        status("resumed", &(self.generation + 1).to_string());
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes().with_title("Claim Lands");
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let browser = web_sys::window().expect("browser window");
            let document = browser.document().expect("document");
            let canvas = document
                .get_element_by_id("game")
                .expect("game canvas")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas element");
            attributes =
                attributes
                    .with_canvas(Some(canvas))
                    .with_inner_size(winit::dpi::LogicalSize::new(
                        browser.inner_width().unwrap().as_f64().unwrap(),
                        browser.inner_height().unwrap().as_f64().unwrap(),
                    ));
        }
        match event_loop.create_window(attributes) {
            Ok(window) => {
                let window = Arc::new(window);
                self.window = Some(window.clone());
                self.initialize_renderer(window);
                self.publish();
            }
            Err(error) => {
                status("error", &error.to_string());
                self.error = Some(error.to_string());
            }
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.generation += 1;
        self.presented = false;
        status("suspended", &self.generation.to_string());
        self.renderer = None;
        self.window = None;
        self.cancel_pointer();
        status("ready", "false");
    }
    fn user_event(&mut self, _: &ActiveEventLoop, event: UserEvent) {
        let UserEvent::Ready(generation, result) = event;
        if generation != self.generation {
            return;
        }
        match result {
            Ok(mut renderer) => {
                // Inputs may regenerate a world while asynchronous GPU creation runs.
                renderer.replace_world(&PlanetAssets::generate(&self.world));
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    renderer.resize(size.width, size.height);
                }
                self.renderer = Some(renderer);
            }
            Err(error) => self.fail_graphics(error),
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self
            .window
            .as_ref()
            .is_none_or(|window| window.id() != window_id)
        {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map_or(1., |w| w.scale_factor()) as f32;
                self.move_pointer(egui::pos2(
                    position.x as f32 / scale,
                    position.y as f32 / scale,
                ));
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => self.press(state == ElementState::Pressed),
            WindowEvent::MouseWheel { delta, .. } => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 80.,
                };
                if !self.pointer_over_controls() {
                    self.distance = (self.distance * (1. - amount * 0.08)).clamp(1.35, 6.);
                    self.publish();
                }
            }
            WindowEvent::Touch(touch) => {
                let scale = self.window.as_ref().map_or(1., |w| w.scale_factor()) as f32;
                let pos = egui::pos2(
                    touch.location.x as f32 / scale,
                    touch.location.y as f32 / scale,
                );
                match touch.phase {
                    TouchPhase::Started => {
                        self.touches.insert(touch.id, pos);
                        self.cursor = pos;
                        if self.touches.len() == 1 {
                            self.press(true);
                        }
                    }
                    TouchPhase::Moved => {
                        self.touches.insert(touch.id, pos);
                        if self.touches.len() == 2 && self.dragging {
                            let points: Vec<_> = self.touches.values().collect();
                            let gap = points[0].distance(*points[1]);
                            if let Some(previous) = self.pinch {
                                self.distance =
                                    (self.distance * previous / gap.max(1.)).clamp(1.35, 6.);
                                self.publish();
                            }
                            self.pinch = Some(gap);
                            self.drag_distance = 100.;
                        } else if self.touches.len() == 1 {
                            self.move_pointer(pos);
                        }
                    }
                    TouchPhase::Ended => {
                        self.cursor = pos;
                        if self.touches.len() == 1 {
                            self.press(false);
                            self.input.events.push(egui::Event::PointerGone);
                        }
                        self.touches.remove(&touch.id);
                        self.pinch = None;
                        if let Some(position) = self.touches.values().next() {
                            self.cursor = *position;
                        }
                    }
                    TouchPhase::Cancelled => self.cancel_pointer(),
                }
            }
            WindowEvent::KeyboardInput { event, .. } => self.keyboard_input(event),
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = egui::Modifiers {
                    alt: state.alt_key(),
                    ctrl: state.control_key(),
                    shift: state.shift_key(),
                    mac_cmd: state.super_key(),
                    command: state.control_key() || state.super_key(),
                };
                self.input
                    .events
                    .push(egui::Event::ModifiersChanged(self.modifiers));
            }
            WindowEvent::Ime(ime) => match ime {
                Ime::Enabled => {}
                Ime::Disabled => self.ime_active = false,
                Ime::Commit(text) => {
                    self.ime_active = false;
                    self.input
                        .events
                        .push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
                }
                Ime::Preedit(text, range) => {
                    self.ime_active = !text.is_empty();
                    let active_range_chars = range.map(|(start, end)| {
                        text[..start].chars().count()..text[..end].chars().count()
                    });
                    self.input
                        .events
                        .push(egui::Event::Ime(egui::ImeEvent::Preedit {
                            text,
                            active_range_chars,
                        }));
                }
            },
            WindowEvent::Focused(focused) => {
                self.input.focused = focused;
                self.input.events.push(egui::Event::WindowFocused(focused));
                if !focused {
                    self.cancel_pointer();
                    self.modifiers = egui::Modifiers::default();
                    self.input
                        .events
                        .push(egui::Event::ModifiersChanged(self.modifiers));
                    self.ime_active = false;
                }
            }
            _ => {}
        }
        if self.error.is_none()
            && self.renderer.is_some()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if self.error.is_none()
            && self.renderer.is_some()
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
    }
}

fn gui_key(key: &Key) -> Option<egui::Key> {
    match key {
        Key::Character(text) => egui::Key::from_name(text),
        Key::Named(name) => Some(match name {
            NamedKey::ArrowDown => egui::Key::ArrowDown,
            NamedKey::ArrowLeft => egui::Key::ArrowLeft,
            NamedKey::ArrowRight => egui::Key::ArrowRight,
            NamedKey::ArrowUp => egui::Key::ArrowUp,
            NamedKey::Escape => egui::Key::Escape,
            NamedKey::Tab => egui::Key::Tab,
            NamedKey::Backspace => egui::Key::Backspace,
            NamedKey::Enter => egui::Key::Enter,
            NamedKey::Insert => egui::Key::Insert,
            NamedKey::Delete => egui::Key::Delete,
            NamedKey::Home => egui::Key::Home,
            NamedKey::End => egui::Key::End,
            NamedKey::PageUp => egui::Key::PageUp,
            NamedKey::PageDown => egui::Key::PageDown,
            NamedKey::Space => egui::Key::Space,
            _ => return None,
        }),
        _ => None,
    }
}

fn parameter(name: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()?
            .location()
            .search()
            .ok()?
            .trim_start_matches('?')
            .split('&')
            .filter_map(|s| s.split_once('='))
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_owned())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = name;
        None
    }
}
fn status(name: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(element) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("game"))
    {
        if name == "error" && value.is_empty() {
            let _ = element.remove_attribute("data-error");
        } else {
            let _ = element.set_attribute(&format!("data-{name}"), value);
        }
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    if !value.is_empty() && matches!(name, "ready" | "error" | "resumed" | "suspended") {
        native_log(&format!("ClaimLands {name}={value}"));
    }
}

#[cfg(target_os = "ios")]
fn native_log(message: &str) {
    eprintln!("{message}");
}

// Android's NativeActivity has no stdout collector. This narrow FFI boundary
// writes only application diagnostics; CString owns a valid terminated buffer.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
fn native_log(message: &str) {
    #[link(name = "log")]
    unsafe extern "C" {
        fn __android_log_write(
            priority: i32,
            tag: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
        ) -> i32;
    }
    if let Ok(message) = std::ffi::CString::new(message) {
        unsafe {
            __android_log_write(4, c"ClaimLands".as_ptr(), message.as_ptr());
        }
    }
}
fn revision() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("game"))
            .and_then(|e| e.get_attribute("data-revision"))
            .unwrap_or_else(|| "local".into())
            .chars()
            .take(8)
            .collect()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        option_env!("CLAIMLANDS_REVISION")
            .unwrap_or("local")
            .chars()
            .take(8)
            .collect()
    }
}
