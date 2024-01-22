use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashSet;
use std::net::{Ipv6Addr, SocketAddrV6};

#[derive(Debug)]
pub enum PlacerError {
    OpenImageFailed(image::ImageError),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
}

impl Placer {
    pub fn new(address: Ipv6Addr) -> Self {
        Self { address }
    }

    #[inline]
    pub fn build_address(&self, pixel: &Pixel) -> Ipv6Addr {
        let address_segments = self.address.segments();

        Ipv6Addr::new(
            address_segments[0],
            address_segments[1],
            address_segments[2],
            address_segments[3],
            (u16::from(pixel.big) + 1) << 12 | pixel.x,
            pixel.y,
            pixel.r,
            (pixel.g << 8) | pixel.b,
        )
    }

    pub fn place_batch(&self, batch: &Vec<Pixel>) {
        let socket = match Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::ICMPV6)) {
            Ok(socket) => socket,
            Err(error) => {
                println!("unable to create socket: {error}");
                return;
            }
        };
        socket.set_nonblocking(true).unwrap();
        socket.set_send_buffer_size(usize::MAX).unwrap();

        for pixel in batch {
            loop {
                if socket
                    .send_to(
                        &[0x80, 0, 0, 0, 0, 0, 0, 0],
                        &SocketAddrV6::new(self.build_address(pixel), 0, 0, 0).into(),
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
                x: u16::try_from(x + x_offset).unwrap(),
                y: u16::try_from(y + y_offset).unwrap(),
                r: u16::from(color[0]),
                g: u16::from(color[1]),
                b: u16::from(color[2]),
                big: false,
            });
        }
    }
    Ok(pixels)
}

pub fn optimize_pixels(pixels: &Vec<Pixel>) -> Vec<Pixel> {
    let mut optimized_pixels = Vec::new();
    let mut ignored = HashSet::new();
    let mut neighbors = Vec::new();

    for pixel in pixels {
        if ignored.contains(pixel) {
            continue;
        }

        let mut big = false;
        neighbors.clear();
        let expected_neighbors = [
            Pixel {
                x: pixel.x + 1,
                y: pixel.y,
                ..*pixel
            },
            Pixel {
                x: pixel.x,
                y: pixel.y + 1,
                ..*pixel
            },
            Pixel {
                x: pixel.x + 1,
                y: pixel.y + 1,
                ..*pixel
            },
        ];
        for neighbor in &expected_neighbors {
            if pixels.contains(neighbor) {
                neighbors.push(*neighbor);
            }
        }
        if neighbors.len() == 3 {
            ignored.extend(neighbors.drain(..));
            big = true;
        }

        optimized_pixels.push(Pixel { big, ..*pixel });
    }
    optimized_pixels
}
