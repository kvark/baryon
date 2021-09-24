#[cfg(feature = "gltf")]
mod gltf;
#[cfg(feature = "obj")]
mod obj;

#[cfg(feature = "gltf")]
pub use self::gltf::load_gltf;
#[cfg(feature = "obj")]
pub use self::obj::load_obj;
