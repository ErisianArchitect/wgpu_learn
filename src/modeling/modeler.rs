use glam::*;

use crate::voxel::vertex::Vertex;

#[derive(Debug, Default, Clone, Copy)]
pub struct PosUV {
    pub pos: Vec3,
    pub uv: Vec2,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PosIndex {
    pub pos: Vec3,
    pub texindex: u32,
}

impl PosUV {
    pub const fn new(pos: Vec3, uv: Vec2) -> Self {
        Self {
            pos,
            uv,
        }
    }

    pub fn upgrade(self, texindex: u32) -> Vertex {
        Vertex::new(self.pos, self.uv, texindex)
    }
}

impl PosIndex {
    pub const fn new(pos: Vec3, index: u32) -> Self {
        Self {
            pos,
            texindex: index,
        }
    }

    pub fn upgrade(self, uv: Vec2) -> Vertex {
        Vertex::new(self.pos, uv, self.texindex)
    }
}

pub struct Modeler {
    pub transform_stack: Vec<Mat4>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

pub struct TextureModeler<'a> {
    pub texture_index: u32,
    pub modeler: &'a mut Modeler,
}

impl Modeler {
    pub fn new() -> Self {
        Self {
            transform_stack: vec![Mat4::IDENTITY],
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn new_transformed(transform: Mat4) -> Self {
        Self {
            transform_stack: vec![transform],
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn get_transform(&self) -> Mat4 {
        if self.transform_stack.len() > 0 {
            self.transform_stack[self.transform_stack.len() - 1]
        } else {
            Mat4::IDENTITY
        }
    }

    pub fn push_transform(&mut self, transform: Mat4) {
        let current = self.get_transform();
        let transform = transform * current;
        self.transform_stack.push(transform);
    }

    pub fn pop_transform(&mut self) {
        if self.transform_stack.pop().is_none() {
            self.transform_stack.push(Mat4::IDENTITY);
        }
    }

    pub fn transform<F: FnOnce(&mut Self)>(&mut self, transform: Mat4, model: F) -> &mut Self {
        self.push_transform(transform);
        model(self);
        self.pop_transform();
        self
    }

    pub fn translate<F: FnOnce(&mut Self)>(&mut self, position: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_translation(position));
        model(self);
        self.pop_transform();
        self
    }

    pub fn scale<F: FnOnce(&mut Self)>(&mut self, scale: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_scale(scale));
        model(self);
        self.pop_transform();
        self
    }

    pub fn rotate<F: FnOnce(&mut Self)>(&mut self, rotation: Quat, model: F) -> &mut Self {
        self.push_transform(Mat4::from_quat(rotation));
        model(self);
        self.pop_transform();
        self
    }

    pub fn rotate_euler<F: FnOnce(&mut Self)>(&mut self, order: EulerRot, rotation: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_euler(order, rotation.x, rotation.y, rotation.z));
        model(self);
        self.pop_transform();
        self
    }

    pub fn scale_rotation_translation<F: FnOnce(&mut Self)>(&mut self, scale: Vec3, rotation: Quat, translation: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_scale_rotation_translation(scale, rotation, translation));
        model(self);
        self.pop_transform();
        self
    }

    pub fn texture_index<F: FnOnce(&mut TextureModeler)>(&mut self, index: u32, model: F) -> &mut Self {
        model(&mut TextureModeler::new(index, self));
        self
    }

    pub fn push_triangle(&mut self, vertices: &[Vertex; 3]) -> &mut Self {
        const ORDER: [u16; 3] = [0, 2, 1];
        let start_index = self.vertices.len() as u16;
        let transform = self.get_transform();
        self.vertices.extend(vertices.map(|v| { Vertex::new(transform.transform_point3(v.position), v.uv, v.texindex) }));
        self.indices.extend(ORDER.map(move |n| start_index + n));
        self
    }

    pub fn push_quad(&mut self, vertices: &[Vertex; 4]) -> &mut Self {
        /*
        0 1
        2 3
        order: 0 2 1 2 3 1
        */
        const ORDER: [u16; 6] = [0, 2, 1, 2, 3, 1];
        let start_index = self.vertices.len() as u16;
        let transform = self.get_transform();
        self.vertices.extend(vertices.map(|v| { Vertex::new(transform.transform_point3(v.position), v.uv, v.texindex) }));
        self.indices.extend(ORDER.clone().map(move |n| start_index + n));
        self
    }

    pub fn push_unit_quad(&mut self, texture_index: u32) -> &mut Self {
        let vertices = [
            PosIndex::new(vec3(0.0, 0.0, 0.0), texture_index), PosIndex::new(vec3(1.0, 0.0, 0.0), texture_index),
            PosIndex::new(vec3(0.0, 0.0, 1.0), texture_index), PosIndex::new(vec3(1.0, 0.0, 1.0), texture_index),
        ];
        self.push_quad_unit_uv(&vertices)
    }

    pub fn push_quad_unit_uv(&mut self, vertices: &[PosIndex; 4]) -> &mut Self {
        let vertices = [
            vertices[0].upgrade(vec2(0.0, 0.0)),
            vertices[1].upgrade(vec2(1.0, 0.0)),
            vertices[2].upgrade(vec2(0.0, 1.0)),
            vertices[3].upgrade(vec2(1.0, 1.0)),
        ];
        self.push_quad(&vertices)
    }

    pub fn push_quad_with_uv_rect(&mut self, vertices: &[PosIndex; 4], uv_min_max: &[Vec2; 2]) -> &mut Self {
        let ul = uv_min_max[0].x;
        let uh = uv_min_max[1].x;
        let vl = uv_min_max[0].y;
        let vh = uv_min_max[1].y;
        let vertices = [
            vertices[0].upgrade(uv_min_max[0]),
            vertices[1].upgrade(vec2(uh, vl)),
            vertices[2].upgrade(vec2(ul, vh)),
            vertices[3].upgrade(uv_min_max[1]),
        ];
        self.push_quad(&vertices)
    }
}

impl<'a> TextureModeler<'a> {
    pub fn new(texture_index: u32, modeler: &'a mut Modeler) -> Self {
        Self {
            texture_index,
            modeler,
        }
    }

    pub fn get_transform(&self) -> Mat4 {
        self.modeler.get_transform()
    }

    pub fn push_transform(&mut self, transform: Mat4) {
        self.modeler.push_transform(transform);
    }

    pub fn pop_transform(&mut self) {
        self.modeler.pop_transform();
    }

    pub fn transform<F: FnOnce(&mut Self)>(&mut self, transform: Mat4, model: F) -> &mut Self {
        self.push_transform(transform);
        model(self);
        self.pop_transform();
        self
    }

    pub fn translate<F: FnOnce(&mut Self)>(&mut self, position: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_translation(position));
        model(self);
        self.pop_transform();
        self
    }

    pub fn scale<F: FnOnce(&mut Self)>(&mut self, scale: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_scale(scale));
        model(self);
        self.pop_transform();
        self
    }

    pub fn rotate<F: FnOnce(&mut Self)>(&mut self, rotation: Quat, model: F) -> &mut Self {
        self.push_transform(Mat4::from_quat(rotation));
        model(self);
        self.pop_transform();
        self
    }

    pub fn rotate_euler<F: FnOnce(&mut Self)>(&mut self, order: EulerRot, rotation: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_euler(order, rotation.x, rotation.y, rotation.z));
        model(self);
        self.pop_transform();
        self
    }

    pub fn scale_rotation_translation<F: FnOnce(&mut Self)>(&mut self, scale: Vec3, rotation: Quat, translation: Vec3, model: F) -> &mut Self {
        self.push_transform(Mat4::from_scale_rotation_translation(scale, rotation, translation));
        model(self);
        self.pop_transform();
        self
    }

    pub fn texture_index<F: FnOnce(&mut TextureModeler)>(&mut self, index: u32, model: F) -> &mut Self {
        self.modeler.texture_index(index, model);
        self
    }

    pub fn push_triangle(&mut self, vertices: &[PosUV; 3]) -> &mut Self {
        let vertices = [
            vertices[0].upgrade(self.texture_index),
            vertices[1].upgrade(self.texture_index),
            vertices[2].upgrade(self.texture_index),
        ];
        self.modeler.push_triangle(&vertices);
        self
    }

    pub fn push_quad(&mut self, vertices: &[PosUV; 4]) -> &mut Self {
        let vertices = [
            vertices[0].upgrade(self.texture_index),
            vertices[1].upgrade(self.texture_index),
            vertices[2].upgrade(self.texture_index),
            vertices[3].upgrade(self.texture_index),
        ];
        self.modeler.push_quad(&vertices);
        self
    }

    pub fn push_unit_quad(&mut self) -> &mut Self {
        self.modeler.push_unit_quad(self.texture_index);
        self
    }

    pub fn push_quad_unit_uv(&mut self, vertices: &[Vec3; 4]) -> &mut Self {
        let vertices = [
            PosIndex::new(vertices[0], self.texture_index),
            PosIndex::new(vertices[1], self.texture_index),
            PosIndex::new(vertices[2], self.texture_index),
            PosIndex::new(vertices[3], self.texture_index),
        ];
        self.modeler.push_quad_unit_uv(&vertices);
        self
    }

    pub fn push_quad_with_uv_rect(&mut self, vertices: &[Vec3; 4], uv_min_max: &[Vec2; 2]) -> &mut Self {
        let vertices = [
            PosIndex::new(vertices[0], self.texture_index),
            PosIndex::new(vertices[1], self.texture_index),
            PosIndex::new(vertices[2], self.texture_index),
            PosIndex::new(vertices[3], self.texture_index),
        ];
        self.modeler.push_quad_with_uv_rect(&vertices, uv_min_max);
        self
    }
}

#[cfg(test)]
mod testing_sandbox {
    // TODO: Remove this sandbox when it is no longer in use.
    use super::*;
    #[test]
    fn sandbox() {
        let mut m = Modeler::new();
        m.texture_index(0, move |m| {
            for y in 0..16 {
                for x in 0..16 {
                    let xf = x as f32;
                    let yf = y as f32;
                    m.translate(vec3(xf, 0.0, yf), move |m| {
                        m.push_unit_quad();
                    });
                }
            }
        });
        println!("{}, {}", m.vertices.len(), m.indices.len());
        println!("{:?}", &m.vertices[4..8]);
    }
}

macro_rules! prototype { ($($_:tt)*) => {} }

prototype!(
    let mut m = Modeler::new();
    m.transformed(positioned(vec3(0.0, 0.0, 0.0)), |m| {
        m.texture_index(0, |m| {

        })
    });
);
