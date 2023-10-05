use ipv6_placer::Pixel;

pub fn draw_pixels(start_x: u32, start_y: u32, end_x: u32, end_y: u32, color: u32) -> Vec<Pixel> {
    let mut pixels = Vec::new();
    for x in start_x..end_x {
        for y in start_y..end_y {
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;
            pixels.push(Pixel {
                x: x as u16,
                y: y as u16,
                r: r as u16,
                g: g as u16,
                b: b as u16,
                big: false,
            })
        }
    }
    pixels
}
