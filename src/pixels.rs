use ipv6_placer::Pixel;

pub fn draw(start_x: u32, start_y: u32, end_x: u32, end_y: u32, color: u32) -> Vec<Pixel> {
    let mut pixels = Vec::new();
    for x in start_x..end_x {
        for y in start_y..end_y {
            let r = (color >> 16) & 0xFF;
            let g = (color >> 8) & 0xFF;
            let b = color & 0xFF;
            pixels.push(Pixel {
                x: u16::try_from(x).unwrap(),
                y: u16::try_from(y).unwrap(),
                r: u16::try_from(r).unwrap(),
                g: u16::try_from(g).unwrap(),
                b: u16::try_from(b).unwrap(),
                big: false,
            });
        }
    }
    pixels
}
