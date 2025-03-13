// Tweening

pub mod f32 {

    pub fn quadratic_in(t: f32) -> f32 {
        t * t
    }

    pub fn quadratic_out(t: f32) -> f32 {
        t * (2.0 - t)
    }

    pub fn quadratic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            -1.0 + (4.0 - 2.0 * t) * t
        }
    }

    pub fn cubic_in(t: f32) -> f32 {
        t.powf(3.0)
    }

    pub fn cubic_out(t: f32) -> f32 {
        let t1 = t - 1.0;
        t1 * t1 * t1 + 1.0
    }

    pub fn cubic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            let t1 = (2.0 * t) - 2.0;
            0.5 * t1 * t1 * t1 + 1.0
        }
    }

    pub fn quartic_in(t: f32) -> f32 {
        t.powf(4.0)
    }

    pub fn quartic_out(t: f32) -> f32 {
        let t1 = t - 1.0;
        1.0 - t1 * t1 * t1 * t1
    }

    pub fn quartic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            8.0 * t * t * t * t
        } else {
            let t1 = t - 1.0;
            1.0 - 8.0 * t1 * t1 * t1 * t1
        }
    }

    pub fn quintic_in(t: f32) -> f32 {
        t.powf(5.0)
    }

    pub fn quintic_out(t: f32) -> f32 {
        let t1 = t - 1.0;
        1.0 + t1 * t1 * t1 * t1 * t1
    }

    pub fn quintic_in_out(t: f32) -> f32 {
        if t < 0.5 {
            16.0 * t * t * t * t * t
        } else {
            let t1 = (2.0 * t) - 2.0;
            0.5 * t1 * t1 * t1 * t1 * t1 + 1.0
        }
    }

    pub fn sine_in(t: f32) -> f32 {
        1.0 - (t * std::f32::consts::FRAC_PI_2).cos()
    }

    pub fn sine_out(t: f32) -> f32 {
        (t * std::f32::consts::FRAC_PI_2).sin()
    }

    pub fn sine_in_out(t: f32) -> f32 {
        0.5 * (1.0 - (std::f32::consts::PI * t).cos())
    }

    pub fn circular_in(t: f32) -> f32 {
        1.0 - (1.0 - t.powf(2.0)).sqrt()
    }

    pub fn circular_out(t: f32) -> f32 {
        (1.0 - (1.0 - t).powf(2.0)).sqrt()
    }

    pub fn circular_in_out(t: f32) -> f32 {
        if t < 0.5 {
            0.5 * (1.0 - (1.0 - (4.0 * t.powf(2.0))).sqrt())
        } else {
            0.5 * ((1.0 - 4.0 * (1.0 - t).powf(2.0)).sqrt() + 1.0)
        }
    }

    pub fn exp_in(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else {
            (2.0_f32).powf(10.0 * (t - 1.0))
        }
    }

    pub fn exp_out(t: f32) -> f32 {
        if t == 1.0 {
            1.0
        } else {
            1.0 - (2.0_f32).powf(-10.0 * t)
        }
    }

    pub fn exp_in_out(t: f32) -> f32 {
        if t == 0.0 {
            0.0
        } else if t == 1.0 {
            1.0
        } else if t < 0.5 {
            0.5 * (2.0_f32).powf(20.0 * t - 10.0)
        } else {
            1.0 - 0.5 * (2.0_f32).powf(-20.0 * t + 10.0)
        }
    }

    pub fn bounce_in(t: f32) -> f32 {
        1.0 - bounce_out(1.0 - t)
    }

    pub fn bounce_out(t: f32) -> f32 {
        if t < 4.0 / 11.0 {
            (121.0 * t * t) / 16.0
        } else if t < 8.0 / 11.0 {
            (363.0 / 40.0 * t * t) - (99.0 / 10.0 * t) + 17.0 / 5.0
        } else if t < 9.0 / 10.0 {
            (4356.0 / 361.0 * t * t) - (35442.0 / 1805.0 * t) + 16061.0 / 1805.0
        } else {
            (54.0 / 5.0 * t * t) - (513.0 / 25.0 * t) + 268.0 / 25.0
        }
    }

    pub fn bounce_in_out(t: f32) -> f32 {
        if t < 0.5 {
            0.5 * bounce_in(t * 2.0)
        } else {
            0.5 * bounce_out(t * 2.0 - 1.0) + 0.5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn easing_test() {
        const BAR: &'static str = "████████████████████████████████████████████████████████████████████████████████████████████████████";
        for i in 0..100 {
            let t = (i as f32) / 100.0;
            let t = super::f32::circular_in(t);
            let c = (100.0 * t) as usize;
            println!("{}", &BAR[..c * 3]);
        }
    }
}