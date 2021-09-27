#[cfg(feature = "gltf")]
mod gltf;
#[cfg(feature = "obj")]
mod obj;

#[cfg(feature = "gltf")]
pub use self::gltf::load_gltf;
#[cfg(feature = "obj")]
pub use self::obj::load_obj;

/// A common ancestor of "sprite sheet", "tile map".
pub struct SpriteMap {
    pub origin: mint::Point2<u16>,
    pub cell_size: mint::Vector2<u16>,
}

impl SpriteMap {
    pub fn at(&self, index: mint::Point2<usize>) -> crate::UvRange {
        let begin = mint::Point2 {
            x: index.x as i16 * self.cell_size.x as i16,
            y: index.y as i16 * self.cell_size.y as i16,
        };
        let end = mint::Point2 {
            x: begin.x + self.cell_size.x as i16,
            y: begin.y + self.cell_size.y as i16,
        };
        begin..end
    }
}
