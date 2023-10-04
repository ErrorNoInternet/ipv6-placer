mod lib;

use clap::{Args, Parser, Subcommand};
use ipv6_placer::{build_pixels_from_image, optimize_pixels, Pixel, Placer};
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    net::{Ipv6Addr, SocketAddrV6},
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
        #[arg(long, default_value_t = 0)]
        start_x: u32,

        #[arg(long, default_value_t = 0)]
        start_y: u32,

        #[arg(long, default_value_t = 1)]
        end_x: u32,

        #[arg(long, default_value_t = 1)]
        end_y: u32,

        #[arg(short, long, default_value_t = 0)]
        color: u32,
    },
}

fn main() {
    let arguments = Arguments::parse();
    let mut max_threads = arguments.threads;
    if max_threads == 0 {
        max_threads = num_cpus::get()
    }

    match arguments.command {
        Commands::DrawImage {
            image,
            image_x_offset,
            image_y_offset,
        } => {
            let image_pixels =
                build_pixels_from_image(&image, image_x_offset, image_y_offset).unwrap();
            let placer = Arc::new(Placer::new(Ipv6Addr::new(
                0x2a01, 0x4f8, 0xc012, 0xf8e6, 0, 0, 0, 0,
            )));
            let active_threads = Arc::new(Mutex::new(0));
            let mut current_batch = Vec::new();
            for pixel in image_pixels {
                current_batch.push(pixel);
                if current_batch.len() >= arguments.batch_size {
                    let placer_arc = placer.clone();
                    let active_threads_arc = active_threads.clone();
                    let current_batch_arc = Arc::new(current_batch.clone());
                    while *active_threads.lock().unwrap() > max_threads {
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
            todo!()
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
