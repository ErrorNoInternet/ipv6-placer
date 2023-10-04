use clap::{Parser, Subcommand};
use ipv6_placer::{build_pixels_from_image, Pixel, Placer};
use std::{
    net::Ipv6Addr,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Parser)]
struct Arguments {
    #[arg(short, long, default_value_t = 32768)]
    batch_size: usize,

    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    #[arg(short, long)]
    no_optimize: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    DrawImage {
        #[arg(short, long)]
        image: String,

        #[arg(long, default_value_t = 0, requires = "image")]
        image_x_offset: u32,

        #[arg(long, default_value_t = 0, requires = "image")]
        image_y_offset: u32,
    },

    #[command(arg_required_else_help = true)]
    DrawPixels {
        #[arg(long)]
        start_x: u32,

        #[arg(long)]
        start_y: u32,

        #[arg(long)]
        end_x: u32,

        #[arg(long)]
        end_y: u32,

        #[arg(short, long, default_value_t = 0)]
        color: u32,
    },
}

fn main() {
    let arguments = Arguments::parse();

    let placer = Arc::new(Placer::new(Ipv6Addr::new(
        0x2a01, 0x4f8, 0xc012, 0xf8e6, 0, 0, 0, 0,
    )));
    match arguments.command {
        Commands::DrawImage {
            image,
            image_x_offset,
            image_y_offset,
        } => {
            let image_pixels = match build_pixels_from_image(&image, image_x_offset, image_y_offset)
            {
                Ok(image_pixels) => image_pixels,
                Err(error) => {
                    println!("unable to open image file: {error:?}");
                    return;
                }
            };
            let active_threads = Arc::new(Mutex::new(0));
            let mut current_batch = Vec::new();
            for pixel in image_pixels {
                current_batch.push(pixel);
                if current_batch.len() >= arguments.batch_size {
                    let placer_arc = placer.clone();
                    let active_threads_arc = active_threads.clone();
                    let current_batch_arc = Arc::new(current_batch.clone());
                    while *active_threads.lock().unwrap() > arguments.threads {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    std::thread::spawn(move || {
                        send_batch(
                            placer_arc,
                            current_batch_arc,
                            active_threads_arc,
                            !arguments.no_optimize,
                        )
                    });
                    *active_threads.lock().unwrap() += 1;
                    current_batch.clear()
                }
            }
            let placer_arc = placer.clone();
            let active_threads_arc = active_threads.clone();
            let current_batch_arc = Arc::new(current_batch.clone());
            std::thread::spawn(move || {
                send_batch(
                    placer_arc,
                    current_batch_arc,
                    active_threads_arc,
                    !arguments.no_optimize,
                )
            });
            *active_threads.lock().unwrap() += 1;
            while *active_threads.lock().unwrap() > 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        Commands::DrawPixels {
            start_x,
            start_y,
            end_x,
            end_y,
            color,
        } => {
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
            let active_threads = Arc::new(Mutex::new(0));
            let mut current_batch = Vec::new();
            for pixel in pixels {
                current_batch.push(pixel);
                if current_batch.len() >= arguments.batch_size {
                    let placer_arc = placer.clone();
                    let active_threads_arc = active_threads.clone();
                    let current_batch_arc = Arc::new(current_batch.clone());
                    while *active_threads.lock().unwrap() > arguments.threads {
                        std::thread::sleep(Duration::from_millis(1))
                    }
                    std::thread::spawn(move || {
                        send_batch(
                            placer_arc,
                            current_batch_arc,
                            active_threads_arc,
                            !arguments.no_optimize,
                        )
                    });
                    *active_threads.lock().unwrap() += 1;
                    current_batch.clear()
                }
            }
            while *active_threads.lock().unwrap() > arguments.threads {
                std::thread::sleep(Duration::from_millis(1))
            }
            let placer_arc = placer.clone();
            let active_threads_arc = active_threads.clone();
            let current_batch_arc = Arc::new(current_batch.clone());
            std::thread::spawn(move || {
                send_batch(
                    placer_arc,
                    current_batch_arc,
                    active_threads_arc,
                    !arguments.no_optimize,
                )
            });
            *active_threads.lock().unwrap() += 1;
            while *active_threads.lock().unwrap() > 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

fn send_batch(
    placer: Arc<Placer>,
    batch: Arc<Vec<Pixel>>,
    active_threads: Arc<Mutex<usize>>,
    optimize: bool,
) {
    placer.place_batch(&batch, optimize);
    *active_threads.lock().unwrap() -= 1;
}
