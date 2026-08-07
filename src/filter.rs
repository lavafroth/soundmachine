use hound::WavReader;
use ndarray::Array2;
use spectrograms::CqtParams;
use spectrograms::Decibels;
use spectrograms::LogParams;
use spectrograms::SpectrogramParams;
use spectrograms::SpectrogramPlanner;
use spectrograms::StftParams;
use spectrograms::WindowType;
use spectrograms::nzu;
use std::collections::HashMap;
use std::error::Error;
use std::num::NonZeroUsize;

pub(crate) fn load_input() -> Result<(Vec<f64>, f64), Box<dyn Error>> {
    let reader = WavReader::open("tests/fixtures/input.wav")?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f64;
    let num_channels = spec.channels as usize;

    let pcm_le_16bit_max = i16::MAX as f64;
    let raw_samples: Vec<f64> = reader
        .into_samples::<i16>()
        .map(|sample| sample.map(|s| s as f64 / pcm_le_16bit_max))
        .collect::<Result<_, _>>()?;

    let mono_samples = if num_channels > 1 {
        raw_samples
            .chunks_exact(num_channels)
            .map(|chunk| chunk.iter().sum::<f64>() / num_channels as f64)
            .collect::<Vec<f64>>()
    } else {
        raw_samples
    };

    Ok((mono_samples, sample_rate))
}

pub(crate) fn cqt(samples: &[f64], sample_rate_hz: f64) -> Result<f64, Box<dyn std::error::Error>> {
    let samples = non_empty_slice::NonEmptySlice::try_new(samples)?;

    // let n_fft = nzu!(2048);
    let hop_size = nzu!(512);

    let bins_per_octave = nzu!(12);
    let n_octaves = nzu!(4);
    let f_min = 65.41;

    let n_fft = NonZeroUsize::try_from(calculate_safe_n_fft(
        f_min,
        sample_rate_hz,
        bins_per_octave.get() as u32,
    ) as usize)?;

    let stft = StftParams::new(n_fft, hop_size, WindowType::Hanning, true)?;
    let cqt_params = CqtParams::new(bins_per_octave, n_octaves, f_min)?;
    let params = SpectrogramParams::new(stft, sample_rate_hz)?;

    let db_floor = LogParams::new(-80.0)?;
    let mut plan = SpectrogramPlanner::new().cqt_plan::<Decibels, f64>(
        &params,
        &cqt_params,
        Some(&db_floor),
    )?;
    let spectrogram = plan.compute(&samples)?;
    let db_matrix = spectrogram.data();

    let f = loudest_frequency(db_matrix, cqt_params.frequencies().as_non_empty_slice());

    Ok(f)
}

pub fn loudest_frequency(chunk: &Array2<f64>, bins: &non_empty_slice::NonEmptySlice<f64>) -> f64 {
    let n_columns = chunk.ncols();

    let mut peak_frequencies = Vec::with_capacity(n_columns);
    let mut how_many_above_energy_threshold = 0;

    for column in chunk.columns() {
        let Some((argmax, max_db)) = column
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(idx, &val)| (idx, val))
        else {
            continue;
        };

        if max_db > -15.0 {
            peak_frequencies.push(argmax);
            how_many_above_energy_threshold += 1;
        }
    }

    if how_many_above_energy_threshold == 0 {
        return f64::NAN;
    }

    let mut frequencies_map = HashMap::new();
    for &freq in &peak_frequencies {
        *frequencies_map.entry(freq).or_insert(0) += 1;
    }

    let Some(stable_center_row) = frequencies_map
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(idx, _)| idx)
    else {
        return f64::NAN;
    };

    bins[stable_center_row]
}

fn calculate_safe_n_fft(fmin: f64, sample_rate: f64, bins_per_octave: u32) -> u32 {
    // Q factor formula taken from librosa
    let q = 1.0 / (2.0f64.powf(1.0 / bins_per_octave as f64) - 1.0);

    // Number of samples required for the lowest frequency filter
    let min_required_samples = (q * sample_rate / fmin) as u32;
    min_required_samples.next_power_of_two()
}

#[cfg(test)]
mod test {
    use crate::filter::*;

    #[test]
    fn mode_frequency_single_file() -> Result<(), Box<dyn std::error::Error>> {
        let (samples, sr) = load_input()?;
        let f = cqt(&samples, sr)?;
        assert!((f - 185.007).abs() < 0.001);
        Ok(())
    }
}
