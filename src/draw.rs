use crate::types;

pub fn draw_dot_grid(
    mmap: &mut memmap2::MmapMut,
    width: u32,
    height: u32,
    config: types::Config,
    grid: &types::Grid,
    current_pos: (u32, u32),
) {
    let bg_color = config.get_bg_color().to_le_bytes(); // BGRA

    let dot_color = config.get_fg_color().to_le_bytes(); // BGRA

    let dot_radius = 2;

    for y in 0..height {
        for x in 0..width {
            let offset = (y * width + x) as usize * 4;
            mmap[offset] = bg_color[0]; // B
            mmap[offset + 1] = bg_color[1]; // G
            mmap[offset + 2] = bg_color[2]; // R
            mmap[offset + 3] = bg_color[3]; // A
        }
    }

    let spacing = config.get_pixels_per_point();

    let grid_width = (width / spacing) + 1;
    let grid_height = (height / spacing) + 1;

    let connection_color = [
        (dot_color[0] as f32 * 0.5) as u8, // B
        (dot_color[1] as f32 * 0.5) as u8, // G
        (dot_color[2] as f32 * 0.5) as u8, // R
        0xff,                              // A
    ];

    (0..grid_height)
        .flat_map(|grid_y| (0..grid_width).map(move |grid_x| (grid_x, grid_y)))
        .for_each(|(grid_x, grid_y)| {
            let visit_count = grid.get_visits(grid_x, grid_y);

            let intensity = (visit_count as f32 / 10.0).min(1.0);
            let r = (dot_color[2] as f32 + (255.0 - dot_color[2] as f32) * intensity) as u8;
            let g = (dot_color[1] as f32 + (200.0 - dot_color[1] as f32) * intensity) as u8;
            let b = (dot_color[0] as f32 + (100.0 - dot_color[0] as f32) * intensity) as u8;

            let (r, g, b) = if (grid_x, grid_y) == current_pos && config.display_active_field() {
                let highlight_colors = config.get_active_color().to_le_bytes(); // BGRA
                (
                    highlight_colors[2],
                    highlight_colors[1],
                    highlight_colors[0],
                )
            } else {
                (r, g, b)
            };

            let dot_color = [b, g, r, 0xff]; // BGRA

            let center_x = grid_x * spacing;
            let center_y = grid_y * spacing;

            if config.connect_dots() && visit_count > 0 {
                if grid_x + 1 < grid_width && grid.get_visits(grid_x + 1, grid_y) > 0 {
                    let neighbor_x = ((grid_x + 1) * spacing) as i32;
                    draw_line(
                        mmap,
                        width,
                        height,
                        center_x as i32,
                        center_y as i32,
                        neighbor_x,
                        center_y as i32,
                        &connection_color,
                    );
                }

                if grid_y + 1 < grid_height && grid.get_visits(grid_x, grid_y + 1) > 0 {
                    let neighbor_y = ((grid_y + 1) * spacing) as i32;
                    draw_line(
                        mmap,
                        width,
                        height,
                        center_x as i32,
                        center_y as i32,
                        center_x as i32,
                        neighbor_y,
                        &connection_color,
                    );
                }
            }

            (-dot_radius..=dot_radius)
                .flat_map(|dy| {
                    (-dot_radius..=dot_radius)
                        .map(move |dx| (dx, dy))
                        .filter(|(dx, dy)| {
                            (dx * dx + dy * dy) as f32 <= (dot_radius * dot_radius) as f32
                        })
                })
                .for_each(|(dx, dy)| {
                    let px = center_x as i32 + dx;
                    let py = center_y as i32 + dy;

                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        let offset = (py as u32 * width + px as u32) as usize * 4;
                        mmap[offset] = dot_color[0]; // B
                        mmap[offset + 1] = dot_color[1]; // G
                        mmap[offset + 2] = dot_color[2]; // R
                        mmap[offset + 3] = dot_color[3]; // A
                    }
                });
        });
}

/// Draw a line between two points using Bresenham's line algorithm
fn draw_line(
    mmap: &mut memmap2::MmapMut,
    width: u32,
    height: u32,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: &[u8; 4],
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let offset = (y as u32 * width + x as u32) as usize * 4;
            mmap[offset] = color[0]; // B
            mmap[offset + 1] = color[1]; // G
            mmap[offset + 2] = color[2]; // R
            mmap[offset + 3] = color[3]; // A
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}
