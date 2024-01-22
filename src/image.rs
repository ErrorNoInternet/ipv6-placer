use ipv6_placer::{build_pixels_from_image, Pixel, PlacerError};

pub fn draw(
    image: &str,
    image_x_offset: u32,
    image_y_offset: u32,
) -> Result<Vec<Pixel>, PlacerError> {
    build_pixels_from_image(image, image_x_offset, image_y_offset)
}
