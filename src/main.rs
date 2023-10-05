use clap::{Parser, Subcommand};
use ipv6_placer::{build_pixels_from_image, optimize_pixels, Pixel, Placer};
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
            if verbose {
                println!("reading directory contents...")
            }
            let directory_contents = match std::fs::read_dir(frames_path) {
                Ok(frame_list) => frame_list,
                Err(error) => {
                    println!("unable to read directory: {error}");
                    return;
                }
            };
            if verbose {
                println!("collecting frame list...")
            }
            let mut frame_list = Vec::new();
            for item in directory_contents {
                if let Ok(item) = item {
                    frame_list.push(item.path().to_str().unwrap().to_string())
                }
            }
            if verbose {
                println!("sorting frame list...")
            }
            frame_list.sort();
            if verbose {
                println!("building first frame pixels...")
            }
            let first_frame_pixels =
                match build_pixels_from_image(&frame_list[0], frame_x_offset, frame_y_offset) {
                    Ok(frame_pixels) => frame_pixels,
                    Err(error) => {
                        println!("unable to open frame: {error:?}");
                        return;
                    }
                };
            if verbose {
                println!("placing first frame pixels...")
            }
            let mut first_frame_pixels = first_frame_pixels;
            if !arguments.no_optimize {
                if verbose {
                    println!("optimizing pixels...")
                }
                first_frame_pixels = optimize_pixels(&first_frame_pixels);
            }
            placer.place_batch(&first_frame_pixels);

            let mut old_frame_pixels = first_frame_pixels;
            for frame in &frame_list[1..] {
                if verbose {
                    println!("building pixels for {frame}...")
                }
                let new_frame_pixels =
                    match build_pixels_from_image(&frame, frame_x_offset, frame_y_offset) {
                        Ok(frame_pixels) => frame_pixels,
                        Err(error) => {
                            println!("unable to open frame: {error:?}");
                            return;
                        }
                    };
                if no_deltas {
                    if verbose {
                        println!("placing new pixels...")
                    }
                    let mut new_frame_pixels = new_frame_pixels.clone();
                    if !arguments.no_optimize {
                        if verbose {
                            println!("optimizing pixels...")
                        }
                        new_frame_pixels = optimize_pixels(&new_frame_pixels);
                    }
                    placer.place_batch(&new_frame_pixels);
                } else {
                    if verbose {
                        println!("finding changed pixels...")
                    }
                    let different_pixels: Vec<Pixel> = new_frame_pixels
                        .iter()
                        .filter(|new_pixel| {
                            old_frame_pixels.iter().any(|old_pixel| {
                                new_pixel.x == old_pixel.x
                                    && new_pixel.y == old_pixel.y
                                    && new_pixel.r != old_pixel.r
                                    && new_pixel.g != old_pixel.g
                                    && new_pixel.b != old_pixel.b
                            })
                        })
                        .map(|item| item.to_owned())
                        .collect();
                    if verbose {
                        println!("placing changed pixels...")
                    }
                    let mut different_pixels = different_pixels;
                    if !arguments.no_optimize {
                        if verbose {
                            println!("optimizing pixels...")
                        }
                        different_pixels = optimize_pixels(&different_pixels);
                    }
                    placer.place_batch(&different_pixels);
                }
                old_frame_pixels = new_frame_pixels;
                if verbose {
                    println!("sleeping for {wait_milliseconds} milliseconds...")
                }
                std::thread::sleep(Duration::from_millis(wait_milliseconds));
            }
            if verbose {
                println!("finished all frames! quitting...")
            }
            return;
        }
        Commands::DrawImage {
            image,
            image_x_offset,
            image_y_offset,
        } => {
            match build_pixels_from_image(&image, image_x_offset, image_y_offset) {
                Ok(image_pixels) => pixels.extend(image_pixels),
                Err(error) => {
                    println!("unable to open image file: {error:?}");
                    return;
                }
            };
        }
        Commands::DrawPixels {
            start_x,
            start_y,
            end_x,
            end_y,
            color,
        } => {
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

fn send_batch(
    placer: Arc<Placer>,
    batch: Arc<Vec<Pixel>>,
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
