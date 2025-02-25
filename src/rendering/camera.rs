use glam::{
    vec2, Mat4, Quat, Vec2, Vec3
};

pub fn rotation_from_look_at(position: Vec3, target: Vec3) -> Vec2 {
    let dir = (target - position).normalize();
    rotation_from_direction(dir)
}

pub fn rotation_from_direction(direction: Vec3) -> Vec2 {
    let yaw = direction.x.atan2(direction.z);
    let pitch = direction.y.asin();
    vec2(pitch, yaw)
}

#[derive(Debug, Clone)]
pub struct Camera {
    position: Vec3,
    rotation: Vec2,
}

impl Camera {
    pub const fn new(
        position: Vec3,
        rotation: Vec2,
    ) -> Self {
        Self {
            position,
            rotation,
        }
    }

    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            rotation: Vec2::ZERO,
        }
    }

    pub fn from_look_at(position: Vec3, target: Vec3) -> Self {
        let rotation = rotation_from_look_at(position, target);
        Self {
            position,
            rotation,
        }
    }

    pub fn from_look_to(position: Vec3, direction: Vec3) -> Self {
        let rotation = rotation_from_direction(direction);
        Self {
            position,
            rotation,
        }
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn set_rotation(&mut self, rotation: Vec2) {
        self.rotation = rotation;
    }

    pub fn translate(&mut self, offset: Vec3) {
        self.position += offset;
    }

    /// Translates relative to camera rotation.
    pub fn translate_rotated(&mut self, offset: Vec3) {
        let rot_quat = self.quat();
        let rot_offset = rot_quat * offset;
        self.translate(rot_offset);
    }

    pub fn look_at(&mut self, target: Vec3) {
        self.rotation = rotation_from_look_at(self.position, target);
    }

    pub fn look_to(&mut self, direction: Vec3) {
        self.rotation = rotation_from_direction(direction);
    }

    pub fn rotate(&mut self, rotation_radians: Vec2) {
        self.rotation += rotation_radians;
        self.rotation.x = self.rotation.x.clamp((-89.0f32).to_radians(), (89.0f32).to_radians());
        self.rotation.y = self.rotation.y.rem_euclid(360.0f32.to_radians());
    }

    /// Returns the quaternion for the [Camera]'s rotation.
    pub fn quat(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, 0.)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let rot_quat = self.quat();
        let up = rot_quat * Vec3::Y;
        let dir = rot_quat * Vec3::NEG_Z;
        Mat4::look_to_rh(self.position, dir, up)
    }
}

#[cfg(test)]
mod tests {
    use glam::{vec3, Vec4};

    use super::*;
    
    #[test]
    fn glam_test() {
        let projection = Mat4::perspective_rh(90.0f32.to_radians(), 1.0, 0.01, 1000.0);
        let mut camera = Camera::from_look_at(Vec3::new(0., 0., 5.), Vec3::ZERO);
        camera.rotate(vec2(15.0f32.to_radians(), 0.0));
        let view = camera.view_matrix();
        // let view = Mat4::look_at_rh(Vec3::new(5.0, 0.0, 0.0), Vec3::Y * 5., Vec3::Y);
        let stage1 = view * Vec4::new(0., 0., 0., 1.);
        let position = projection * stage1;
        let ndc = vec3((position.x / position.w) * 1024.0, (position.y / position.w) * 1024.0, position.z / position.w);
        println!("{ndc:?} {}", ((position.x / position.w) * 16384.0) as i32);
    }
}
