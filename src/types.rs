use std::os::fd::AsFd;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::WaylandSurface,
    shell::wlr_layer::{self, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, QueueHandle, globals,
    protocol::{wl_buffer, wl_output, wl_shm, wl_shm_pool, wl_surface},
};

/// The config file format
#[derive(facet::Facet, Debug, Clone)]
pub struct Config {
    /// How many walks should be performed per minute
    #[facet(default = 30.0)]
    walks_per_minute: f32,
    /// How many pixels one grid point should cover
    #[facet(default = 20)]
    pixels_per_point: u32,
    /// Size of each individual dot in pixels
    #[facet(default = 2)]
    dot_radius: u32,
    /// Background color in ARGB format
    #[facet(default = 0xff1a1a1au32)]
    bg_color: u32,
    /// Foreground color in ARGB format
    #[facet(default = 0xff606060u32)]
    fg_color: u32,
    /// The currently active field
    #[facet(default = 0xffff0000u32)]
    active_color: u32,
}

/// Needs to be manually implemented because facets default only happens when
/// serializing, not for the Default impl.
impl Default for Config {
    fn default() -> Self {
        Config {
            walks_per_minute: 30.0,
            pixels_per_point: 20,
            dot_radius: 2,
            bg_color: 0xff1a1a1au32,
            fg_color: 0xff606060u32,
            active_color: 0xffff0000u32,
        }
    }
}

impl Config {
    /// Get the walks per second
    pub fn walks_per_second(&self) -> f32 {
        self.walks_per_minute / 60.0
    }

    pub fn get_dot_radius(&self) -> u32 {
        self.dot_radius
    }

    pub fn get_bg_color(&self) -> u32 {
        self.bg_color
    }

    pub fn get_fg_color(&self) -> u32 {
        self.fg_color
    }

    pub fn get_pixels_per_point(&self) -> u32 {
        self.pixels_per_point
    }

    pub fn get_active_color(&self) -> u32 {
        self.active_color
    }

    pub fn get_walks_per_minute(&self) -> f32 {
        self.walks_per_minute
    }
}

pub struct WalkState {
    grid_width: u32,
    grid_height: u32,
    current_pos: (u32, u32),
    needs_update: bool,
}

impl WalkState {
    pub fn new(grid_width: u32, grid_height: u32) -> Self {
        WalkState {
            grid_width,
            grid_height,
            current_pos: (0, 0),
            needs_update: false,
        }
    }

    pub fn get_current_pos(&self) -> (u32, u32) {
        self.current_pos
    }

    pub fn needs_update(&self) -> bool {
        self.needs_update
    }

    pub fn get_width(&self) -> u32 {
        self.grid_width
    }

    pub fn get_height(&self) -> u32 {
        self.grid_height
    }

    pub fn set_pos(&mut self, x: u32, y: u32) {
        self.current_pos = (x, y);
    }

    pub fn clear_update_flag(&mut self) {
        self.needs_update = false;
    }

    /// Sets needs_update to true, regardless of the previous value
    pub fn set_needs_update(&mut self) {
        self.needs_update = true;
    }
}

/// Represents the grid of dots with visit counts
pub struct Grid {
    width: u32,
    height: u32,
    visits: Vec<u8>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height) as usize;
        Grid {
            width,
            height,
            visits: vec![0; size],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let size = (width * height) as usize;
        self.visits.resize(size, 0);
        self.visits.fill(0);
    }

    pub fn visit(&mut self, x: u32, y: u32) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.visits[idx] = self.visits[idx].saturating_add(1);
        }
    }

    pub fn get_visits(&self, x: u32, y: u32) -> u8 {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.visits[idx]
        } else {
            0
        }
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }
}

/// Stores application state
pub struct App {
    config: Config,
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    shm_state: Shm,
    layer_surface: Option<wlr_layer::LayerSurface>,
    width: u32,
    height: u32,
    configured: bool,
    pool: Option<wl_shm_pool::WlShmPool>,
    grid: Grid,
    current_pos: (u32, u32),
    needs_redraw: bool,
    file: std::fs::File,
    mmap: Option<memmap2::MmapMut>,
}

impl App {
    pub fn new(global_list: &globals::GlobalList, qh: &QueueHandle<Self>) -> Self {
        let file = tempfile::tempfile().expect("Failed to create tempfile");
        file.lock().expect("Failed to lock tempfile");

        Self {
            config: Config::default(),
            registry_state: RegistryState::new(global_list),
            output_state: OutputState::new(global_list, qh),
            compositor_state: CompositorState::bind(global_list, qh)
                .expect("Failed to bind compositor"),
            shm_state: Shm::bind(global_list, qh).expect("Failed to bind shm"),
            layer_surface: None,
            width: 0,
            height: 0,
            configured: false,
            pool: None,
            grid: Grid::new(0, 0),
            current_pos: (0, 0),
            needs_redraw: false,
            file: tempfile::tempfile().expect("Failed to create temp file"),
            mmap: None,
        }
    }

    pub fn create_surface(&mut self, qh: &QueueHandle<Self>, globals: &globals::GlobalList) {
        let surface = self.compositor_state.create_surface(qh);
        let layer_shell =
            wlr_layer::LayerShell::bind(globals, qh).expect("Failed to bind layer shell");
        let layer_surface = layer_shell.create_layer_surface(
            qh,
            surface,
            wlr_layer::Layer::Background,
            Some("walk_bg"),
            None,
        );

        layer_surface.set_anchor(wlr_layer::Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(wlr_layer::KeyboardInteractivity::None);
        layer_surface.commit();

        self.layer_surface = Some(layer_surface);
    }

    pub fn set_config(&mut self, config: Config) {
        self.config = config;
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn is_configured(&self) -> bool {
        self.configured
    }

    pub fn get_current_pos(&self) -> (u32, u32) {
        self.current_pos
    }

    pub fn set_pos(&mut self, x: u32, y: u32) {
        self.current_pos = (x, y);
    }

    pub fn set_needs_redraw(&mut self) {
        self.needs_redraw = true;
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn needs_no_redraw(&mut self) {
        self.needs_redraw = false;
    }

    pub fn get_grid(&self) -> &Grid {
        &self.grid
    }

    /// Draw a new frame.
    ///
    /// # Safety
    /// We use unsafe for mapping a file mutably into memory. The underlying file is
    /// locked by default and there should be no program that randomly writes to any
    /// tempfile. If you have a suggestion on how to handle this safer, feel free to
    /// open an issue.
    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        if !self.configured || self.width == 0 || self.height == 0 {
            return;
        }

        let layer_surface = match &self.layer_surface {
            Some(s) => s,
            None => {
                return;
            }
        };

        let width = self.width as i32;
        let height = self.height as i32;
        let stride = width * 4;
        let size = stride * height;

        if self.mmap.is_none() {
            self.mmap =
                Some(unsafe { memmap2::MmapMut::map_mut(&self.file).expect("Failed to map file") });
        }

        self.grid.visit(self.current_pos.0, self.current_pos.1);

        crate::draw::draw_dot_grid(
            self.mmap.as_mut().unwrap(),
            self.width,
            self.height,
            self.config.clone(),
            &self.grid,
            self.current_pos,
        );

        if self.pool.is_none() {
            self.pool = Some(
                self.shm_state
                    .wl_shm()
                    .create_pool(self.file.as_fd(), size, qh, ()),
            );
        }

        let buffer = self.pool.as_ref().unwrap().create_buffer(
            0,
            width,
            height,
            stride,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        let wl_surface = layer_surface.wl_surface();
        wl_surface.attach(Some(&buffer), 0, 0);
        wl_surface.damage_buffer(0, 0, width, height);
        wl_surface.commit();
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        println!("Layer surface closed");
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = configure.new_size.0;
        self.height = configure.new_size.1;

        if let Err(e) = self.file.set_len((self.width * 4 * self.height) as u64) {
            eprintln!("Failed to set tempfile length: {e}");
        };

        if self.width == 0 || self.height == 0 {
            self.width = 1920;
            self.height = 1080;
        }

        println!("Display size: {}x{}", self.width, self.height);

        let grid_width = (self.width / self.config.pixels_per_point) + 1;
        let grid_height = (self.height / self.config.pixels_per_point) + 1;
        self.grid.resize(grid_width, grid_height);
        self.current_pos = (grid_width / 2, grid_height / 2);

        println!(
            "Grid initialized: {}x{} (center: {:?})",
            grid_width, grid_height, self.current_pos
        );

        self.configured = true;

        self.draw(qh);
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers![OutputState];
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_layer!(App);
delegate_registry!(App);

wayland_client::delegate_noop!(App: ignore wl_shm_pool::WlShmPool);
wayland_client::delegate_noop!(App: ignore wl_buffer::WlBuffer);
