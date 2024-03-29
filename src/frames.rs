use ipv6_placer::{build_pixels_from_image, optimize_pixels, Pixel, Placer};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

pub fn draw(
    verbose: bool,
    no_optimize: bool,
    batch_size: usize,
    threads: usize,
    placer: &Arc<Placer>,
    frames_path: String,
    frame_x_offset: u32,
    frame_y_offset: u32,
    no_deltas: bool,
    wait_milliseconds: u64,
) {
    if verbose {
        println!("reading directory contents...");
    }
    let directory_contents = match std::fs::read_dir(frames_path) {
        Ok(frame_list) => frame_list,
        Err(error) => {
            println!("unable to read directory: {error}");
            return;
        }
    };
    if verbose {
        println!("collecting frame list...");
    }
    let mut frame_list = Vec::new();
    for item in directory_contents.flatten() {
        frame_list.push(item.path().to_str().unwrap().to_string());
    }
    if frame_list.is_empty() {
        println!("not enough frames!");
        return;
    }

    if verbose {
        println!("sorting frame list...");
    }
    frame_list.sort();
    if verbose {
        println!("building first frame pixels...");
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
        println!("placing first frame pixels...");
    }
    let mut first_frame_pixels = first_frame_pixels;
    if !no_optimize {
        if verbose {
            println!("optimizing pixels...");
        }
        first_frame_pixels = optimize_pixels(&first_frame_pixels);
    }
    placer.place_batch(&first_frame_pixels);

    let mut old_frame_pixels = first_frame_pixels;
    for frame in &frame_list[1..] {
        if verbose {
            println!("building pixels for {frame}...");
        }
        let new_frame_pixels = match build_pixels_from_image(frame, frame_x_offset, frame_y_offset)
        {
            Ok(frame_pixels) => frame_pixels,
            Err(error) => {
                println!("unable to open frame: {error:?}");
                return;
            }
        };
        if no_deltas {
            if verbose {
                println!("placing new pixels...");
            }
            let mut new_frame_pixels = new_frame_pixels.clone();
            if !no_optimize {
                if verbose {
                    println!("optimizing pixels...");
                }
                new_frame_pixels = optimize_pixels(&new_frame_pixels);
            }
            placer.place_batch(&new_frame_pixels);
        } else {
            if verbose {
                println!("finding changed pixels...");
            }
            let different_pixels = Arc::new(Mutex::new(Vec::with_capacity(512 * 512)));
            let active_threads = Arc::new(Mutex::new(0));
            let mut current_batch = Vec::with_capacity(batch_size);
            for new_pixel in &new_frame_pixels {
                current_batch.push(*new_pixel);
                if current_batch.len() > batch_size {
                    if verbose {
                        println!("launching new thread to find changed pixels...");
                    }
                    while *active_threads.lock().unwrap() >= threads {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    let different_pixels_arc = different_pixels.clone();
                    let active_threads_arc = active_threads.clone();
                    let current_batch_arc = current_batch.clone();
                    let old_frame_pixels_arc = old_frame_pixels.clone();
                    std::thread::spawn(move || {
                        let old_frame_pixels_map: HashMap<(u16, u16), (u16, u16, u16)> =
                            old_frame_pixels_arc
                                .iter()
                                .map(|pixel| ((pixel.x, pixel.y), (pixel.r, pixel.g, pixel.b)))
                                .collect();
                        let different_pixels: Vec<Pixel> = current_batch_arc
                            .iter()
                            .filter_map(|new_pixel| {
                                if let Some(old_pixel) =
                                    old_frame_pixels_map.get(&(new_pixel.x, new_pixel.y))
                                {
                                    if new_pixel.r != old_pixel.0
                                        || new_pixel.g != old_pixel.1
                                        || new_pixel.b != old_pixel.2
                                    {
                                        Some(new_pixel.to_owned())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();
                        different_pixels_arc
                            .lock()
                            .unwrap()
                            .extend(different_pixels);
                        *active_threads_arc.lock().unwrap() -= 1;
                        if verbose {
                            println!("thread finished!");
                        }
                    });
                    *active_threads.lock().unwrap() += 1;
                    current_batch.clear();
                }
            }
            while *active_threads.lock().unwrap() >= threads {
                std::thread::sleep(Duration::from_millis(1));
            }
            let different_pixels_arc = different_pixels.clone();
            let active_threads_arc = active_threads.clone();
            let current_batch_arc = current_batch.clone();
            let old_frame_pixels_arc = old_frame_pixels.clone();
            std::thread::spawn(move || {
                let different_pixels: Vec<Pixel> = current_batch_arc
                    .iter()
                    .filter(|new_pixel| {
                        old_frame_pixels_arc.iter().any(|old_pixel| {
                            new_pixel.x == old_pixel.x
                                && new_pixel.y == old_pixel.y
                                && new_pixel.r != old_pixel.r
                                && new_pixel.g != old_pixel.g
                                && new_pixel.b != old_pixel.b
                        })
                    })
                    .map(std::borrow::ToOwned::to_owned)
                    .collect();
                different_pixels_arc
                    .lock()
                    .unwrap()
                    .extend(different_pixels);
                *active_threads_arc.lock().unwrap() -= 1;
                if verbose {
                    println!("thread finished!");
                }
            });
            *active_threads.lock().unwrap() += 1;
            while *active_threads.lock().unwrap() > 0 {
                std::thread::sleep(Duration::from_millis(1));
            }
            let mut different_pixels = different_pixels.lock().unwrap().to_owned();
            if !no_optimize {
                if verbose {
                    println!("optimizing pixels...");
                }
                different_pixels = optimize_pixels(&different_pixels);
            }
            if verbose {
                println!("placing changed pixels...");
            }
            placer.place_batch(&different_pixels);
        }
        old_frame_pixels = new_frame_pixels;
        if verbose {
            println!("sleeping for {wait_milliseconds} milliseconds...");
        }
        std::thread::sleep(Duration::from_millis(wait_milliseconds));
    }
    if verbose {
        println!("finished all frames! quitting...");
    }
}
