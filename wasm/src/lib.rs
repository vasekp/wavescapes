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

struct Instance {
    rng: rand::rngs::SmallRng,
    params: [Params; 2],
    generator: [Generator; 2],
    fix_counter: u32,
    fix_counter_ceil: u32,
}

struct Params {
    herm: [Mat; ITER],
    unit: Mat,
}

struct Generator {
    cx_step: [Complex<f32>; DIM],
    par_step: f32,
    cx: [Complex<f32>; DIM],
}

#[wasm_bindgen]
impl Instance {
    fn new(sample_rate: f32) -> Instance {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(
                (random() * 2.0f64.powi(f64::MANTISSA_DIGITS as i32)) as u64);
        let params = [Params::new(&mut rng), Params::new(&mut rng)];
        let dt1 = FREQ / sample_rate * std::f32::consts::TAU;
        let dt2 = VAR_RATE / sample_rate;
        let generator = [Generator::new(dt1, dt2), Generator::new(dt1, dt2)];
        Instance {
            rng,
            params,
            generator,
            fix_counter: 0,
            fix_counter_ceil: (sample_rate as u32) / (SAMPLES as u32),
        }
    }

    pub fn new_handle(sample_rate: u32) -> usize {
        let bx = Box::new(Instance::new(sample_rate as f32));
        Box::leak(bx) as *mut Instance as usize
    }

    unsafe fn from_handle(handle: usize) -> &'static mut Self {
        unsafe { &mut *(handle as *mut Instance) }
    }
}

impl Params {
    fn new(rng: &mut (impl Rng + SeedableRng)) -> Params {
        let dist = Uniform::new(-1., 1.).unwrap();
        let mut herm = [Default::default(); ITER];
        for ix in 0..ITER {
            herm[ix] = fix_herm(Mat::from_fn(|_, _| Complex::new(rng.sample(dist), rng.sample(dist))));
        }
        let unit = fix_unit(Mat::from_fn(|_, _| Complex::new(rng.sample(dist), rng.sample(dist))));
        Params { herm, unit }
    }

    fn evolve(&mut self, dt: f32) {
        let i_dt = Complex::new(0.0, dt);
        for ix in 1..ITER {
            self.herm[ix] += (self.herm[ix - 1] * self.herm[ix] - self.herm[ix] * self.herm[ix - 1]) * i_dt;
        }
        self.unit += self.herm[ITER - 1] * self.unit * i_dt;
    }

    fn normalize(&mut self) {
        for mx in &mut self.herm {
            *mx = fix_herm(*mx);
        }
        self.unit = fix_unit(self.unit);
    }

    fn mutate(&mut self, rng: &mut (impl Rng + SeedableRng)) {
        let dist = Uniform::new(-1., 1.).unwrap();
        self.herm[0] = fix_herm(Mat::from_fn(|_, _|
            Complex::new(rng.sample(dist), rng.sample(dist))));
    }
}

impl Generator {
    fn new(dt1: f32, dt2: f32) -> Generator {
        let cx_step = MTP.map(|m| Complex::new(0.0, m * dt1).exp());
        let cx = [1.0.into(); DIM];
        Generator { cx_step, par_step: dt2, cx }
    }

    fn generate(&mut self, data: &mut [f32], params: &mut Params) {
        params.evolve((SAMPLES as f32) * self.par_step);
        for x in data {
            let mut res: Complex<f32> = 0.0.into();
            for ix in 0..DIM {
                self.cx[ix] *= self.cx_step[ix];
                res += self.cx[ix] * params.unit[ix];
            }
            *x = res.re / DIVIDER;
        }
    }

    fn normalize(&mut self) {
        for z in &mut self.cx {
            *z /= z.abs();
        }
    }
}

#[wasm_bindgen]
pub fn process(left: &mut [f32], right: &mut [f32], handle: usize) -> () {
    let inst = unsafe { Instance::from_handle(handle) };
    assert!(left.len() == SAMPLES);
    assert!(right.len() == SAMPLES);
    inst.generator[0].generate(left, &mut inst.params[0]);
    inst.generator[1].generate(right, &mut inst.params[1]);
    inst.fix_counter += 1;
    if inst.fix_counter == inst.fix_counter_ceil {
        inst.params[0].normalize();
        inst.params[1].normalize();
        inst.generator[0].normalize();
        inst.generator[1].normalize();
        // use this opportunity for more variation
        inst.params[0].mutate(&mut inst.rng);
        inst.params[1].mutate(&mut inst.rng);
        inst.fix_counter = 0;
    }
}

#[wasm_bindgen]
pub fn get_sample(left: &mut [f32], right: &mut [f32], handle: usize) -> () {
    let inst = unsafe { Instance::from_handle(handle) };
    let len = left.len();
    assert!(right.len() == left.len());
    let mut generator = Generator::new(4.0 * std::f32::consts::TAU / (len as f32), 0.0);
    generator.generate(left, &mut inst.params[0]);
    let mut generator = Generator::new(4.0 * std::f32::consts::TAU / (len as f32), 0.0);
    generator.generate(right, &mut inst.params[1]);
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
