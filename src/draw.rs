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

    (0..grid_height)
        .flat_map(|grid_y| (0..grid_width).map(move |grid_x| (grid_x, grid_y)))
        .for_each(|(grid_x, grid_y)| {
            let visit_count = grid.get_visits(grid_x, grid_y);

            let intensity = (visit_count as f32 / 10.0).min(1.0);
            let r = (dot_color[2] as f32 + (255.0 - dot_color[2] as f32) * intensity) as u8;
            let g = (dot_color[1] as f32 + (200.0 - dot_color[1] as f32) * intensity) as u8;
            let b = (dot_color[0] as f32 + (100.0 - dot_color[0] as f32) * intensity) as u8;

            let (r, g, b) = if (grid_x, grid_y) == current_pos {
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
        }
    }
}
