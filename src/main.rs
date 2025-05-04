use audio::decode;
use plotters::prelude::*;

// Made for the sake of testing
// Not gonna include this in final app
fn plot_waveform(samples: &[i16], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new(output_path, (1024, 300)).into_drawing_area();
    root.fill(&WHITE)?;

    let max = *samples.iter().max().unwrap() as i32;
    let min = *samples.iter().min().unwrap() as i32;

    let mut chart = ChartBuilder::on(&root)
        .caption("Audio Waveform", ("sans-serif", 20))
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..samples.len(), min..max)?;

    chart.configure_mesh().draw()?;

    chart.draw_series(LineSeries::new(
        samples.iter().enumerate().map(|(x, &y)| (x, y as i32)),
        &RED,
    ))?;

    Ok(())
}

fn main() {
    let result = decode("audio.mp3");
    match result {
        Ok(v) => {
            plot_waveform(&v, "waveform.png");
        }
        Err(e) => eprintln!("e: {}", e),
    }
}
