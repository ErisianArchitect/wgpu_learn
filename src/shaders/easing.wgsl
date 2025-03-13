fn quadratic(t: f32) -> f32 {
    return pow(t, 2.0);
}

fn cubic(t: f32) -> f32 {
    return pow(t, 3.0);
}

fn quartic(t: f32) -> f32 {
    return pow(t, 4.0);
}

fn quintic(t: f32) -> f32 {
    return pow(t, 5.0);
}

const EASING_PI: f32 = 3.141592653589793;
const EASING_FRAC_PI_2: f32 = EASING_PI / 2.0;

fn sine_in(t: f32) -> f32 {
    return 1.0 - cos(t * EASING_FRAC_PI_2);
}

fn sine_out(t: f32) -> f32 {
    return sin(t * EASING_FRAC_PI_2);
}

fn sine_in_out(t: f32) -> f32 {
    return 0.5 * (1.0 - cos(EASING_PI * t));
}

fn circular_in(t: f32) -> f32 {
    return 1.0 - sqrt(1.0 - pow(t, 2.0));
}

fn circular_out(t: f32) -> f32 {
    return sqrt(1.0 - pow(1.0 - t, 2.0));
}

fn circular_in_out(t: f32) -> f32 {
    if t < 0.5 {
        return 0.5 * (1.0 - sqrt(1.0 - (4.0 * pow(t, 2.0))));
    } else {
        return 0.5 * (sqrt(1.0 - 4.0 * pow(1.0 - t, 2.0)) + 1.0);
    }
}