use eframe::egui::{Rect, pos2, vec2};
use mandelbrot_test::mandel_set::Domain;
use std::time::Instant;

fn main() {
    let resolution = [2048, 2048];
    let max_iterations = 200; // High enough to show parallel advantages
    let num_restrictions = 3;
    let num_runs_per_restriction = 5;

    println!(
        "\n=== Starting Release Performance Benchmark (Res: {}x{}) ===",
        resolution[0], resolution[1]
    );
    println!("Number of random restrictions: {}", num_restrictions);
    println!(
        "Iterations per restriction:  {}\n",
        num_runs_per_restriction
    );

    for i in 0..num_restrictions {
        // Generate a random viewport near typical mandelbrot regions
        let x_0 = rand::random::<f32>() * 2.0 - 1.0; // [-1.0, 1.0]
        let y_0 = rand::random::<f32>() * 2.0 - 1.0; // [-1.0, 1.0]
        let d_x = rand::random::<f32>() * 1.0 - 0.5; // [-0.5, 0.5]
        let d_y = rand::random::<f32>() * 1.0 - 0.5; // [-0.5, 0.5]

        let restriction = Rect::from_center_size(pos2(x_0, y_0), vec2(d_x, d_y));

        println!("Restriction {}: {:?}", i + 1, restriction);

        let mut seq_durations = Vec::new();
        let mut par_durations = Vec::new();

        for _ in 0..num_runs_per_restriction {
            // Sequential
            let domain_seq = Domain::new(restriction, resolution);
            let start_seq = Instant::now();
            let _ = domain_seq.generate_image_by_rayon(max_iterations, false);
            seq_durations.push(start_seq.elapsed());

            // Parallel
            let domain_par = Domain::new(restriction, resolution);
            let start_par = Instant::now();
            let _ = domain_par.generate_image_by_rayon(max_iterations, true);
            par_durations.push(start_par.elapsed());
        }

        let avg_seq =
            seq_durations.iter().sum::<std::time::Duration>() / num_runs_per_restriction as u32;
        let avg_par =
            par_durations.iter().sum::<std::time::Duration>() / num_runs_per_restriction as u32;
        let speedup = avg_seq.as_secs_f64() / avg_par.as_secs_f64();

        println!("  Sequential Time: {:?}", avg_seq);
        println!("  Parallel Time:   {:?}", avg_par);
        println!("  Speedup:         {:.2}x", speedup);
        println!("--------------------------------------------------");
    }
}
