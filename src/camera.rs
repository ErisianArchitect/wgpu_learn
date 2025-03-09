use glam::{
    vec2, Mat4, Quat, Vec2, Vec3, Vec4, Vec4Swizzles
};

use crate::math::ray::Ray3;

pub fn rotation_from_look_at(position: Vec3, target: Vec3) -> Vec2 {
    let dir = (target - position).normalize();
    rotation_from_direction(dir)
}

pub fn rotation_from_direction(direction: Vec3) -> Vec2 {
    let yaw = (-direction.x).atan2(-direction.z);
    let pitch = direction.y.asin();
    vec2(pitch, yaw)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MoveType {
    /// Absolute movement. No rotation of the translation vector.
    Absolute,
    /// Free movement. Rotates the translation vector with the camera.
    Free,
    /// Planar movement. Rotates the translation vector with the angle around the Y axis.
    Planar,
}

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Vec2,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub const fn new(
        position: Vec3,
        rotation: Vec2,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            position,
            rotation,
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    /// Creates an unrotated camera at the given position.
    pub fn at(
        position: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            position,
            rotation: Vec2::ZERO,
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    pub fn from_look_at(
        position: Vec3,
        target: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let rotation = rotation_from_look_at(position, target);
        Self {
            position,
            rotation,
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    /// `look_to` means to point in the same direction as the given `direction` vector.
    pub fn from_look_to(
        position: Vec3,
        direction: Vec3,
        fov: f32,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        let rotation = rotation_from_direction(direction);
        Self {
            position,
            rotation,
            fov,
            aspect_ratio,
            z_near,
            z_far,
        }
    }

    pub fn rotate_vec(&self, v: Vec3) -> Vec3 {
        let rot = self.quat();
        rot * v
    }

    /// Rotates vector around the Y axis.
    pub fn rotate_vec_y(&self, v: Vec3) -> Vec3 {
        let rot = self.y_quat();
        rot * v
    }

    pub fn up(&self) -> Vec3 {
        self.rotate_vec(Vec3::Y)
    }

    pub fn down(&self) -> Vec3 {
        self.rotate_vec(Vec3::NEG_Y)
    }

    pub fn left(&self) -> Vec3 {
        self.rotate_vec(Vec3::NEG_X)
    }

    pub fn right(&self) -> Vec3 {
        self.rotate_vec(Vec3::X)
    }

    pub fn forward(&self) -> Vec3 {
        self.rotate_vec(Vec3::NEG_Z)
    }

    pub fn backward(&self) -> Vec3 {
        self.rotate_vec(Vec3::Z)
    }

    pub fn pan_forward(&self) -> Vec3 {
        self.rotate_vec_y(Vec3::NEG_Z)
    }

    pub fn pan_backward(&self) -> Vec3 {
        self.rotate_vec_y(Vec3::Z)
    }

    pub fn adv_move(&mut self, move_type: MoveType, translation: Vec3) {
        match move_type {
            MoveType::Absolute => self.translate(translation),
            MoveType::Free => self.translate_rotated(translation),
            MoveType::Planar => self.translate_planar(translation),
        }
    }

    pub fn translate(&mut self, translation: Vec3) {
        self.position += translation;
    }

    /// Translates relative to camera rotation.
    pub fn translate_rotated(&mut self, translation: Vec3) {
        if translation.length_squared() > 0.000001 {
            let rot_quat = self.quat();
            let rot_offset = rot_quat * translation;
            self.translate(rot_offset);
        }
    }

    /// For planar camera translation.
    pub fn translate_planar(&mut self, translation: Vec3) {
        if translation.length_squared() > 0.000001 {
            self.translate(self.rotate_vec_y(translation))
        }
    }

    pub fn look_at(&mut self, target: Vec3) {
        self.rotation = rotation_from_look_at(self.position, target);
    }

    pub fn look_to(&mut self, direction: Vec3) {
        self.rotation = rotation_from_direction(direction);
    }

    pub fn rotate(&mut self, rotation_radians: Vec2) {
        self.rotation += rotation_radians;
        self.rotation.x = self.rotation.x.clamp(-90f32.to_radians(), 90f32.to_radians());
        self.rotation.y = self.rotation.y.rem_euclid(360f32.to_radians());
    }

    pub fn rotate_x(&mut self, radians: f32) {
        self.rotation.x += radians;
        self.rotation.x = self.rotation.x.clamp(-90f32.to_radians(), 90f32.to_radians());
    }

    pub fn rotate_y(&mut self, radians: f32) {
        self.rotation.y += radians;
        self.rotation.y = self.rotation.y.rem_euclid(360f32.to_radians());
    }

    /// Returns the quaternion for the [Camera]'s rotation.
    pub fn quat(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.rotation.y, self.rotation.x, 0.)
    }

    pub fn x_quat(&self) -> Quat {
        Quat::from_axis_angle(Vec3::X, self.rotation.x)
    }

    pub fn y_quat(&self) -> Quat {
        Quat::from_axis_angle(Vec3::Y, self.rotation.y)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let rot_quat = self.quat();
        let up = rot_quat * Vec3::Y;
        let dir = rot_quat * Vec3::NEG_Z;
        Mat4::look_to_rh(self.position, dir, up)
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect_ratio, self.z_near, self.z_far)
    }

    pub fn projection_view_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    pub fn world_to_clip(&self, pos: Vec3) -> Vec4 {
        let view_proj = self.projection_view_matrix();
        let pos_w = Vec4::new(pos.x, pos.y, pos.z, 1.0);
        view_proj * pos_w
    }

    pub fn world_to_clip_ncd(&self, pos: Vec3) -> Vec3 {
        let clip = self.world_to_clip(pos);
        clip.xyz() / clip.w
    }
    
    pub fn normalized_screen_to_ray(&self, screen_pos: Vec2) -> Ray3 {
        let inv_proj_view = self.projection_view_matrix().inverse();

        let near_point = inv_proj_view * Vec4::new(screen_pos.x, -screen_pos.y, 0.0, 1.0);
        let near_point = near_point.xyz() / near_point.w;
        let far_point = inv_proj_view * Vec4::new(screen_pos.x, -screen_pos.y, self.z_far, 1.0);
        let far_point = far_point.xyz() / far_point.w;

        let direction = (near_point - far_point).normalize();

        Ray3::new(self.position, direction)
    }
}

#[cfg(test)]
mod tests {
    use glam::{vec3, Vec4};

    use super::*;

    #[test]
    fn radians_test() {
        assert_eq!(-90f32.to_radians(), (-90f32).to_radians());
    }
    
    #[test]
    fn glam_test() {
        let projection = Mat4::perspective_rh(90.0f32.to_radians(), 1.0, 0.01, 1000.0);
        let mut camera = Camera::from_look_at(Vec3::new(0., 0., 5.), Vec3::ZERO, 45f32.to_radians(), 1280./720., 0.01, 1000.0);
        camera.rotate(vec2(15.0f32.to_radians(), 0.0));
        let view = camera.view_matrix();
        // let view = Mat4::look_at_rh(Vec3::new(5.0, 0.0, 0.0), Vec3::Y * 5., Vec3::Y);
        let stage1 = view * Vec4::new(0., 0., 0., 1.);
        let position = projection * stage1;
        let ndc = vec3((position.x / position.w) * 1024.0, (position.y / position.w) * 1024.0, position.z / position.w);
        println!("{ndc:?} {}", ((position.x / position.w) * 16384.0) as i32);
    }
}
