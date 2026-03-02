use mandelbrot_test::mandel_set::Restriction;
use num::Complex;
use std::time::Instant;

fn main() {
    let resolution = [1000, 1000];
    let max_iterations = 5; // High enough to show parallel advantages
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
        let x_0 = rand::random::<f64>() * 4.0 - 2.0; // [-2.0, 2.0]
        let y_0 = rand::random::<f64>() * 4.0 - 2.0; // [-2.0, 2.0]
        let x_1 = rand::random::<f64>() * 4.0 - 2.0; // [-2.0, 2.0]
        let y_1 = rand::random::<f64>() * 4.0 - 2.0; // [-2.0, 2.0]

        let restriction = Restriction::from_two_points(
            Complex::new(x_0, y_0),
            Complex::new(x_1, y_1),
            resolution[0],
            resolution[1],
        );

        println!("Restriction {}: {:?}", i + 1, restriction);
        let domain = restriction.into_domain();

        let mut seq_durations = Vec::new();
        let mut par_durations = Vec::new();

        for _ in 0..num_runs_per_restriction {
            // Sequential
            let start_seq = Instant::now();
            let _ = domain.clone().calculate_image(max_iterations);
            seq_durations.push(start_seq.elapsed());

            // Parallel
            let start_par = Instant::now();
            let _ = domain
                .clone()
                .calculate_image_by_rayon(max_iterations, true);
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
