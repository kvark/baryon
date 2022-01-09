use std::ops;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Space {
    position: glam::Vec3,
    scale: f32,
    orientation: glam::Quat,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            scale: 1.0,
            orientation: glam::Quat::IDENTITY,
        }
    }
}

impl Space {
    pub(super) fn combine(&self, other: &Self) -> Self {
        Self {
            scale: self.scale * other.scale,
            orientation: self.orientation * other.orientation,
            position: self.scale * (self.orientation * other.position) + self.position,
        }
    }

    fn inverse(&self) -> Self {
        let scale = 1.0 / self.scale;
        let orientation = self.orientation.inverse();
        let position = -scale * (orientation * self.position);
        Self {
            position,
            scale,
            orientation,
        }
    }

    fn to_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::splat(self.scale),
            self.orientation,
            self.position,
        )
    }
}

impl<T> super::ObjectBuilder<'_, T> {
    //TODO: should we accept `V: Into<mint::...>` here?
    pub fn position(&mut self, position: mint::Vector3<f32>) -> &mut Self {
        self.node.local.position = position.into();
        self
    }

    pub fn scale(&mut self, scale: f32) -> &mut Self {
        self.node.local.scale = scale;
        self
    }

    pub fn orientation_around(&mut self, axis: mint::Vector3<f32>, angle_deg: f32) -> &mut Self {
        self.node.local.orientation =
            glam::Quat::from_axis_angle(axis.into(), angle_deg.to_radians());
        self
    }

    pub fn orientation(&mut self, quat: mint::Quaternion<f32>) -> &mut Self {
        self.node.local.orientation = quat.into();
        self
    }

    pub fn look_at(&mut self, target: mint::Vector3<f32>, up: mint::Vector3<f32>) -> &mut Self {
        /* // This path just doesn't work well
        let dir = (glam::Vec3::from(target) - self.node.local.position).normalize();
        self.node.local.orientation = glam::Quat::from_rotation_arc(-glam::Vec3::Z, dir);
            * glam::Quat::from_rotation_arc(glam::Vec3::Y, up.into());
        let temp = glam::Quat::from_rotation_arc(glam::Vec3::Y, up.into();
        let new_dir = temp * -glam::Vec3::Z;
        self.node.local.orientation = glam::Quat::from_rotation_arc(-glam::Vec3::Z, dir);
        */

        let affine = glam::Affine3A::look_at_rh(self.node.local.position, target.into(), up.into());
        let (_, rot, _) = affine.inverse().to_scale_rotation_translation();
        // translation here is expected to match `self.node.local.position`
        self.node.local.orientation = rot;

        /* // Blocked on https://github.com/bitshifter/glam-rs/issues/235
        let dir = self.node.local.position - glam::Vec3::from(target);
        let f = dir.normalize();
        let s = glam::Vec3::from(up).cross(f).normalize();
        let u = f.cross(s);
        self.node.local.orientation = glam::Quat::from_rotation_axes(s, u, f);
        */
        self
    }
}

impl super::Node {
    pub fn get_position(&self) -> mint::Vector3<f32> {
        self.local.position.into()
    }
    pub fn set_position(&mut self, pos: mint::Vector3<f32>) {
        self.local.position = pos.into();
    }
    pub fn pre_move(&mut self, offset: mint::Vector3<f32>) {
        let other = Space {
            position: offset.into(),
            scale: 1.0,
            orientation: glam::Quat::IDENTITY,
        };
        self.local = other.combine(&self.local);
    }
    pub fn post_move(&mut self, offset: mint::Vector3<f32>) {
        self.local.position += glam::Vec3::from(offset);
    }

    pub fn get_rotation(&self) -> (mint::Vector3<f32>, f32) {
        let (axis, angle) = self.local.orientation.to_axis_angle();
        (axis.into(), angle.to_degrees())
    }
    pub fn set_rotation(&mut self, axis: mint::Vector3<f32>, angle_deg: f32) {
        self.local.orientation = glam::Quat::from_axis_angle(axis.into(), angle_deg.to_radians());
    }
    pub fn pre_rotate(&mut self, axis: mint::Vector3<f32>, angle_deg: f32) {
        self.local.orientation = self.local.orientation
            * glam::Quat::from_axis_angle(axis.into(), angle_deg.to_radians());
    }
    pub fn post_rotate(&mut self, axis: mint::Vector3<f32>, angle_deg: f32) {
        let other = Space {
            position: glam::Vec3::ZERO,
            scale: 1.0,
            orientation: glam::Quat::from_axis_angle(axis.into(), angle_deg.to_radians()),
        };
        self.local = other.combine(&self.local);
    }

    pub fn get_scale(&self) -> f32 {
        self.local.scale
    }
    pub fn set_scale(&mut self, scale: f32) {
        self.local.scale = scale;
    }
}

#[derive(Debug)]
pub struct RawSpace {
    pub pos_scale: [f32; 4],
    pub rot: [f32; 4],
}

impl From<Space> for RawSpace {
    fn from(s: Space) -> Self {
        Self {
            pos_scale: [s.position.x, s.position.y, s.position.z, s.scale],
            rot: s.orientation.into(),
        }
    }
}

impl RawSpace {
    pub(super) fn to_space(&self) -> Space {
        Space {
            position: glam::Vec3::new(self.pos_scale[0], self.pos_scale[1], self.pos_scale[2]),
            scale: self.pos_scale[3],
            orientation: glam::Quat::from_array(self.rot),
        }
    }

    pub fn inverse_matrix(&self) -> mint::ColumnMatrix4<f32> {
        self.to_space().inverse().to_matrix().into()
    }
}

#[derive(Clone, Debug)]
pub enum Projection {
    Orthographic {
        /// The center of the projection.
        center: mint::Vector2<f32>,
        /// Vertical extent from the center point. The height is double the extent.
        /// The width is derived from the height based on the current aspect ratio.
        extent_y: f32,
    },
    Perspective {
        /// Vertical field of view, in degrees.
        /// Note: the horizontal FOV is computed based on the aspect.
        fov_y: f32,
    },
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub projection: Projection,
    /// Specify the depth range as seen by the camera.
    /// `depth.start` maps to 0.0, and `depth.end` maps to 1.0.
    pub depth: ops::Range<f32>,
    pub node: super::NodeRef,
    pub background: super::Color,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            projection: Projection::Orthographic {
                center: mint::Vector2 { x: 0.0, y: 0.0 },
                extent_y: 1.0,
            },
            depth: 0.0..1.0,
            node: super::NodeRef::default(),
            background: super::Color::default(),
        }
    }
}

impl Camera {
    pub fn projection_matrix(&self, aspect: f32) -> mint::ColumnMatrix4<f32> {
        let matrix = match self.projection {
            Projection::Orthographic { center, extent_y } => {
                let extent_x = aspect * extent_y;
                glam::Mat4::orthographic_rh(
                    center.x - extent_x,
                    center.x + extent_x,
                    center.y - extent_y,
                    center.y + extent_y,
                    self.depth.start,
                    self.depth.end,
                )
            }
            Projection::Perspective { fov_y } => {
                let fov = fov_y.to_radians();
                if self.depth.end == f32::INFINITY {
                    assert!(self.depth.start.is_finite());
                    glam::Mat4::perspective_infinite_rh(fov, aspect, self.depth.start)
                } else if self.depth.start == f32::INFINITY {
                    glam::Mat4::perspective_infinite_reverse_rh(fov, aspect, self.depth.end)
                } else {
                    glam::Mat4::perspective_rh(fov, aspect, self.depth.start, self.depth.end)
                }
            }
        };
        matrix.into()
    }
}
