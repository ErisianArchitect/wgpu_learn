pub mod transform;
pub mod ray;
pub mod average;

#[inline(always)]
pub const fn morton6(index: u32) -> u32 {
    let step1 = (index | (index << 6)) & 0b0000111000000111;
    let step2 = (step1 | (step1 << 2)) & 0b0011001000011001;
    let step3 = (step2 | (step2 << 2)) & 0b1001001001001001;
    return step3;
}

#[inline(always)]
pub const fn morton6_3(x: u32, y: u32, z: u32) -> u32 {
    morton6(x) | (morton6(y) << 1) | (morton6(z) << 2)
}