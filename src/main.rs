mod frames;
mod image;
mod pixels;

use clap::{Parser, Subcommand};
use ipv6_placer::{optimize_pixels, Pixel, Placer};
use std::{
    net::Ipv6Addr,
    sync::{
        atomic::{AtomicUsize, Ordering::SeqCst},
        Arc,
    },
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
    forever: bool,

    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Frames {
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
    Image {
        #[arg(short, long)]
        image: String,

        #[arg(long, default_value_t = 0, requires = "image")]
        image_x_offset: u32,

        #[arg(long, default_value_t = 0, requires = "image")]
        image_y_offset: u32,
    },

    #[command(arg_required_else_help = true)]
    Pixels {
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
        Commands::Frames {
            frames_path,
            frame_x_offset,
            frame_y_offset,
            no_deltas,
            wait_milliseconds,
        } => {
            frames::draw(
                verbose,
                arguments.no_optimize,
                arguments.batch_size,
                arguments.threads,
                &placer,
                frames_path,
                frame_x_offset,
                frame_y_offset,
                no_deltas,
                wait_milliseconds,
            );
            return;
        }

        Commands::Image {
            image,
            image_x_offset,
            image_y_offset,
        } => match image::draw(&image, image_x_offset, image_y_offset) {
            Ok(image_pixels) => pixels.extend(image_pixels),
            Err(error) => {
                eprintln!("unable to read image: {error:?}");
                return;
            }
        },

        Commands::Pixels {
            start_x,
            start_y,
            end_x,
            end_y,
            color,
        } => pixels.extend(pixels::draw(start_x, start_y, end_x, end_y, color)),
    }

    let active_threads = Arc::new(AtomicUsize::new(0));
    loop {
        for batch in pixels.chunks(arguments.batch_size) {
            let active_threads_arc = active_threads.clone();
            let placer_arc = placer.clone();
            let batch_arc = batch.to_owned();
            while active_threads.load(SeqCst) > arguments.threads {
                std::thread::sleep(Duration::from_millis(1));
            }
            std::thread::spawn(move || {
                place_batch(
                    &placer_arc,
                    batch_arc,
                    &active_threads_arc,
                    !arguments.no_optimize,
                );
            });
        }

        if !arguments.forever {
            break;
        }
    }
    while active_threads.load(SeqCst) > 0 {
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn place_batch(
    placer: &Arc<Placer>,
    batch: Vec<Pixel>,
    active_threads: &Arc<AtomicUsize>,
    optimize: bool,
) {
    active_threads.fetch_add(1, SeqCst);

    let mut pixels = batch;
    if optimize {
        pixels = optimize_pixels(&pixels);
    }
    placer.place_batch(&pixels);

    active_threads.fetch_sub(1, SeqCst);
}
