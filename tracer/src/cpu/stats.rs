use crate::common::scene::Color;

/// Calculates the variance of a value online with only a fixed amount of memory using
/// [Welford's algorithm](https://en.wikipedia.org/wiki/Algorithms_for_calculating_variance#Welford's_online_algorithm).
///
/// Call `update` for each sample obtained.
#[derive(Debug, Default, Clone)]
pub struct ColorVarianceEstimator {
    pub count: u32,
    pub mean: Color,
    m2: Color,
}

impl ColorVarianceEstimator {
    /// Updates the internal state given a new sample.
    pub fn update(&mut self, value: Color) {
        self.count += 1;
        let delta = value - self.mean;
        self.mean += delta / (self.count as f32);
        let delta_2 = value - self.mean;
        self.m2 += delta * delta_2;
    }

    /// Returns the current variance.
    pub fn variance(&self) -> Option<Color> {
        if self.count >= 2 {
            Some(self.m2 / (self.count as f32))
        } else {
            return None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn variance_estimator() {
        let xs = [0.5, 0.1, 0.4, 0.8, 0.3, 0.9, 0.8, 0.4, 0.2];
        let mut estimator = ColorVarianceEstimator::default();

        for (i, &x) in xs.iter().enumerate() {
            estimator.update(Color::new(x, 0.0, 0.0));
            let expected_count = (i + 1) as u32;

            if i + 1 >= 2 {
                let mean = estimator.mean.red;
                let variance = estimator.variance()
                    .expect("Variance should be available once we have at least two samples")
                    .red;

                let expected_mean = xs[0..i + 1].iter().copied().sum::<f32>() / (expected_count as f32);
                let expected_variance = xs[0..i + 1].iter().map(|&x| (x - expected_mean).powi(2)).sum::<f32>() / (expected_count as f32);

                assert_eq!(expected_count, estimator.count);
                assert!(expected_mean - mean < 0.00001);
                assert!(expected_variance - variance < 0.00001);
            }
        }
    }
}