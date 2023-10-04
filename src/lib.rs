use std::net::{Ipv6Addr, SocketAddrV6};

use socket2::{Domain, Protocol, Socket, Type};

#[derive(Debug)]
pub enum PlacerError {
    OpenImageFailed(image::ImageError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pixel {
    pub x: u16,
    pub y: u16,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub big: bool,
}

#[derive(Clone, Debug)]
pub struct Placer {
    address: Ipv6Addr,
    persistent: bool,
}

impl Placer {
    pub fn new(address: Ipv6Addr) -> Self {
        Self {
            address,
            persistent: false,
        }
    }

    pub fn build_address(&self, pixel: &Pixel) -> Ipv6Addr {
        let address_segments = self.address.segments();
        let addr = Ipv6Addr::new(
            address_segments[0],
            address_segments[1],
            address_segments[2],
            address_segments[3],
            (pixel.big as u16 + 1) << 12 | pixel.x,
            pixel.y,
            pixel.r as u16,
            (pixel.g << 8) | pixel.b,
        );
        addr
    }

    pub fn place_batch(&self, batch: &Vec<Pixel>, optimize: bool) {
        let socket = match Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6)) {
            Ok(socket) => socket,
            Err(error) => {
                println!("unable to create raw socket: {error}");
                return;
            }
        };
        socket.set_nonblocking(true).unwrap();
        socket.set_send_buffer_size(usize::MAX).unwrap();

        let mut pixels = batch.clone();
        if optimize {
            pixels = optimize_pixels(&pixels);
        };
        for pixel in pixels {
            loop {
                if socket
                    .send_to(
                        &[0x80, 0, 0, 0, 0, 0, 0, 0],
                        &SocketAddrV6::new(self.build_address(&pixel), 0, 0, 0).into(),
                    )
                    .is_ok()
                {
                    break;
                }
            }
        }
    }
}

pub fn build_pixels_from_image(
    image_path: &str,
    x_offset: u32,
    y_offset: u32,
) -> Result<Vec<Pixel>, PlacerError> {
    let mut pixels = Vec::new();
    let image_object = match image::open(image_path) {
        Ok(image_object) => image_object.into_rgba8(),
        Err(error) => return Err(PlacerError::OpenImageFailed(error)),
    };
    for (x, y, color) in image_object.enumerate_pixels() {
        let color = color.0;
        if color[3] != 0 {
            pixels.push(Pixel {
                x: (x + x_offset) as u16,
                y: (y + y_offset) as u16,
                r: color[0] as u16,
                g: color[1] as u16,
                b: color[2] as u16,
                big: false,
            })
        }
    }
    Ok(pixels)
}

pub fn optimize_pixels(pixels: &Vec<Pixel>) -> Vec<Pixel> {
    let mut optimized_pixels = Vec::new();
    let mut ignored = Vec::new();
    for pixel in pixels {
        if ignored.contains(pixel) {
            continue;
        }
        let mut neighbors = Vec::new();
        let mut big = false;

        let expected_neighbor0 = Pixel {
            x: pixel.x + 1,
            y: pixel.y,
            r: pixel.r,
            g: pixel.g,
            b: pixel.b,
            big: pixel.big,
        };
        if pixels.contains(&expected_neighbor0) {
            neighbors.push(expected_neighbor0)
        }
        let expected_neighbor1 = Pixel {
            x: pixel.x,
            y: pixel.y + 1,
            r: pixel.r,
            g: pixel.g,
            b: pixel.b,
            big: pixel.big,
        };
        if pixels.contains(&expected_neighbor1) {
            neighbors.push(expected_neighbor1)
        }
        let expected_neighbor2 = Pixel {
            x: pixel.x + 1,
            y: pixel.y + 1,
            r: pixel.r,
            g: pixel.g,
            b: pixel.b,
            big: pixel.big,
        };
        if pixels.contains(&expected_neighbor2) {
            neighbors.push(expected_neighbor2)
        }

        if neighbors.len() == 3 {
            ignored.extend(neighbors);
            big = true;
        }
        optimized_pixels.push(Pixel {
            x: pixel.x,
            y: pixel.y,
            r: pixel.r,
            g: pixel.g,
            b: pixel.b,
            big,
        })
    }
    optimized_pixels
}
