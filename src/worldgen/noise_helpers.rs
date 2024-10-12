
use noise::{NoiseFn, Perlin};
use valence::math::DVec3;

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1.0 - t) + b * t
}

pub fn lerpstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if x <= edge0 {
        0.0
    } else if x >= edge1 {
        1.0
    } else {
        (x - edge0) / (edge1 - edge0)
    }
}

pub fn generator(noise: &Perlin, p: DVec3, octaves: u32, lacunarity: f64, persistence: f64) -> f64 {
    let mut freq = 1.0;
    let mut amp = 1.0;
    let mut amp_sum = 0.0;
    let mut sum = 0.0;

    for _ in 0..octaves {
        let n = noise.get((p * freq).to_array());
        sum += n * amp;
        amp_sum += amp;

        freq *= lacunarity;
        amp *= persistence;
    }

    // Scale the output to [0, 1]
    sum / amp_sum
}

pub fn range_noise(noise: &Perlin, p: DVec3) -> f64 {
    let lower = generator(noise, p, 16, 1.0, 0.95);
    let upper = generator(noise, p, 16, 1.0, 0.95);
    let selector = generator(noise, p, 8, 12.75, 1.0) + 0.5;

    let selector = lerpstep(0.0, 1.0, selector);
    lerp(lower, upper, selector)
}
