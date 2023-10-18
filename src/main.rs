mod draw_frames;
mod draw_image;
mod draw_pixels;

use clap::{Parser, Subcommand};
use ipv6_placer::{optimize_pixels, Pixel, Placer};
use std::{
    net::Ipv6Addr,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Debug, Parser)]
struct Arguments {
    #[arg(short, long, default_value_t = 4096)]
    batch_size: usize,

    #[arg(short, long, default_value_t = num_cpus::get())]
    threads: usize,

    #[arg(short, long)]
    no_optimize: bool,

    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    DrawFrames {
        #[arg(short, long)]
        frames_path: String,

        #[arg(long, default_value_t = 0, requires = "frames_path")]
        frame_x_offset: u32,

        #[arg(long, default_value_t = 0, requires = "frames_path")]
        frame_y_offset: u32,

        #[arg(short, long, requires = "frames_path")]
        no_deltas: bool,

        #[arg(short, long, default_value_t = 1000, requires = "frames_path")]
        wait_milliseconds: u64,
    },

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
    let verbose = arguments.verbose;

    let placer = Arc::new(Placer::new(Ipv6Addr::new(
        0x2a01, 0x4f8, 0xc012, 0xf8e6, 0, 0, 0, 0,
    )));
    let mut pixels = Vec::new();
    match arguments.command {
        Commands::DrawFrames {
            frames_path,
            frame_x_offset,
            frame_y_offset,
            no_deltas,
            wait_milliseconds,
        } => {
            draw_frames::draw_frames(
                verbose,
                arguments.no_optimize,
                arguments.batch_size,
                arguments.threads,
                placer,
                frames_path,
                frame_x_offset,
                frame_y_offset,
                no_deltas,
                wait_milliseconds,
            );
            return;
        }
        Commands::DrawImage {
            image,
            image_x_offset,
            image_y_offset,
        } => match draw_image::draw_image(image, image_x_offset, image_y_offset) {
            Ok(image_pixels) => pixels.extend(image_pixels),
            Err(error) => {
                println!("unable to read image: {error:?}");
                return;
            }
        },
        Commands::DrawPixels {
            start_x,
            start_y,
            end_x,
            end_y,
            color,
        } => pixels.extend(draw_pixels::draw_pixels(
            start_x, start_y, end_x, end_y, color,
        )),
    }

    let active_threads = Arc::new(Mutex::new(0));
    let mut current_batch = Vec::new();
    for pixel in pixels {
        current_batch.push(pixel);
        if current_batch.len() >= arguments.batch_size {
            let placer_arc = placer.clone();
            let active_threads_arc = active_threads.clone();
            let current_batch_arc = current_batch.clone();
            while *active_threads.lock().unwrap() > arguments.threads {
                std::thread::sleep(Duration::from_millis(1))
            }
            std::thread::spawn(move || {
                place_batch(
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
    let current_batch_arc = current_batch.clone();
    std::thread::spawn(move || {
        place_batch(
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

fn place_batch(
    placer: Arc<Placer>,
    batch: Vec<Pixel>,
    active_threads: Arc<Mutex<usize>>,
    optimize: bool,
) {
    let mut pixels = batch;
    if optimize {
        pixels = optimize_pixels(&pixels).into();
    }
    placer.place_batch(&pixels);
    *active_threads.lock().unwrap() -= 1;
}
