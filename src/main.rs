use wayland_client::{Connection, globals::registry_queue_init};

use types::{App, Config};

pub mod draw;
pub mod types;
pub mod utils;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config
    let config_path = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("walk_bg")
        .join("config.toml");
    let res = std::fs::read_to_string(config_path);
    let config = if let Ok(file) = res
        && let Ok(cfg) = facet_toml::from_str(&file)
    {
        cfg
    } else {
        println!("Failed to parse config file, using defaults");
        Config::default()
    };

    // Connect to the Wayland server
    let conn = Connection::connect_to_env()?;

    // Create event queue and handle registry initialization
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();

    let mut app = App::new(&globals, &qh);
    app.set_config(config);

    app.create_surface(&qh, &globals);

    while !app.is_configured() {
        event_queue.blocking_dispatch(&mut app)?;
    }

    let walk_interval =
        std::time::Duration::from_secs_f32(60.0 / app.get_config().get_walks_per_minute());
    let mut last_walk = std::time::Instant::now();

    // Run the event loop
    println!("Running background layer shell surface...");
    loop {
        if app.is_configured() && last_walk.elapsed() >= walk_interval {
            // Perform a walk step
            let (x, y) = app.get_current_pos();
            let (new_x, new_y) = utils::random_walk_step(
                x,
                y,
                app.get_grid().get_width(),
                app.get_grid().get_height(),
            );

            app.set_pos(new_x, new_y);

            // Redraw
            app.draw(&qh);

            last_walk = std::time::Instant::now();
        }

        event_queue.flush()?;
        match conn.prepare_read() {
            Some(guard) => {
                let _ = guard.read();
                event_queue.dispatch_pending(&mut app)?;
            }
            None => {
                event_queue.dispatch_pending(&mut app)?;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
