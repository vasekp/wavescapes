use wasm_bindgen::prelude::*;
use nalgebra::*;
use rand::{Rng, distr::Uniform, SeedableRng};

// Compile with:
// RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack build --target web

const MTP: [f32; 7] = [1.0, 1.25, 1.5, 2.0, 2.5, 3.0, 4.0];
const DIM: usize = MTP.len();
type Mat = SMatrix::<Complex<f32>, DIM, DIM>;

const ITER: usize = 3;

const FREQ: f32 = 100.0;
const VAR_RATE: f32 = 3.0;
const SAMPLES: usize = 128;
const DIVIDER: f32 = approx_sqrt(DIM as f32);

struct State {
    rng: rand::rngs::SmallRng,
    herm: [Mat; ITER],
    unit: Mat,
    cx_step: [Complex<f32>; DIM],
    i_dt: Complex<f32>,
    cx: [Complex<f32>; DIM],
    fix_counter: u32,
    fix_counter_ceil: u32,
    phase: f32,
}

#[wasm_bindgen]
impl State {
    fn new(sample_rate: f32) -> State {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(
                (random() * 2.0f64.powi(f64::MANTISSA_DIGITS as i32)) as u64);
        let dist = Uniform::new(-1., 1.).unwrap();
        let mut herm = [Default::default(); ITER];
        for ix in 0..ITER {
            herm[ix] = fix_herm(Mat::from_fn(|_, _| Complex::new(rng.sample(dist), rng.sample(dist))));
        }
        let unit = fix_unit(Mat::from_fn(|_, _| Complex::new(rng.sample(dist), rng.sample(dist))));
        let bf = FREQ / sample_rate * std::f32::consts::TAU;
        let cx_step = MTP.map(|m| Complex::new(0.0, m * bf).exp());
        State {
            rng,
            herm,
            unit,
            cx_step,
            i_dt: Complex::new(0.0, VAR_RATE / sample_rate * (SAMPLES as f32)),
            cx: [1.0.into(); DIM],
            fix_counter: 0,
            fix_counter_ceil: (sample_rate as u32) / (SAMPLES as u32),
            phase: 0.0,
        }
    }

    pub fn new_handle(sample_rate: u32) -> usize {
        let bx = Box::new(State::new(sample_rate as f32));
        Box::leak(bx) as *mut State as usize
    }

    unsafe fn from_handle(handle: usize) -> &'static mut Self {
        unsafe { &mut *(handle as *mut State) }
    }
}

#[wasm_bindgen]
pub fn process(left: &mut [f32], right: &mut [f32], handle: usize) -> () {
    let state = unsafe { State::from_handle(handle) };
    for ix in 1..ITER {
        state.herm[ix] += (state.herm[ix - 1] * state.herm[ix]
            - state.herm[ix] * state.herm[ix - 1]) * state.i_dt;
    }
    state.unit += state.herm[ITER - 1] * state.unit * state.i_dt;
    state.phase += state.i_dt.im;
    assert!(left.len() == SAMPLES);
    assert!(right.len() == SAMPLES);
    for sample in 0..SAMPLES {
        let mut res1: Complex<f32> = 0.0.into();
        for ix in 0..DIM {
            state.cx[ix] *= state.cx_step[ix];
        }
        for ix in 0..DIM {
            res1 += state.cx[ix] * state.unit[ix];
        }
        let mut res2: Complex<f32> = 0.0.into();
        for ix in 0..DIM {
            res2 += state.cx[ix] * state.unit[ix];
        }
        res2 *= Complex::new(0.0, state.phase).exp();
        left[sample] = res1.re / DIVIDER;
        right[sample] = res2.re / DIVIDER;
    }
    state.fix_counter += 1;
    if state.fix_counter == state.fix_counter_ceil {
        for z in &mut state.cx {
            *z /= z.abs();
        }
        for mx in &mut state.herm {
            *mx = fix_herm(*mx);
        }
        state.unit = fix_unit(state.unit);
        state.fix_counter = 0;
        // use this opportunity for more variation
        let dist = Uniform::new(-1., 1.).unwrap();
        state.herm[0] = fix_herm(Mat::from_fn(|_, _|
            Complex::new(state.rng.sample(dist), state.rng.sample(dist))));
    }
}

#[wasm_bindgen]
pub fn get_sample(left: &mut [f32], right: &mut [f32], handle: usize) -> () {
    let state = unsafe { State::from_handle(handle) };
    let len = left.len();
    assert!(right.len() == left.len());
    let dt = 4.0 * std::f32::consts::TAU / (len as f32);
    let cx_step = MTP.map(|m| Complex::new(0.0, m * dt).exp());
    let mut cx: [Complex<f32>; DIM] = [1.0.into(); DIM];
    for sample in 0..len {
        let mut res1: Complex<f32> = 0.0.into();
        for ix in 0..DIM {
            cx[ix] *= cx_step[ix];
        }
        for ix in 0..DIM {
            res1 += cx[ix] * state.unit[ix];
        }
        let mut res2: Complex<f32> = 0.0.into();
        for ix in 0..DIM {
            res2 += cx[ix] * state.unit[ix];
        }
        res2 *= Complex::new(0.0, state.phase).exp();
        left[sample] = res1.re / DIVIDER;
        right[sample] = res2.re / DIVIDER;
    }
}

fn fix_herm(mut m: Mat) -> Mat {
    m = (m + m.adjoint()) / Complex::from(2.0);
    m -= Mat::identity() * m.trace() / Complex::from(DIM as f32);
    m /= m.ad_mul(&m).trace().sqrt();
    m
}

fn fix_unit(m: Mat) -> Mat {
    let svd = m.svd_unordered(true, true);
    svd.u.unwrap() * svd.v_t.unwrap()
}

const fn approx_sqrt(x: f32) -> f32 {
    let mut y = 1.0;
    y = (y + x / y) / 2.;
    y = (y + x / y) / 2.;
    y = (y + x / y) / 2.;
    y
}

#[wasm_bindgen(js_namespace = Math)]
extern "C" {
    fn random() -> f64;
}
