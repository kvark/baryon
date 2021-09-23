#[cfg(feature = "obj")]
mod obj;

#[cfg(feature = "obj")]
pub use self::obj::load_obj;
