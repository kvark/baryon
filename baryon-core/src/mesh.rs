use std::marker::PhantomData;

pub struct Vertex<T> {
    mesh: super::MeshRef,
    _phantom: PhantomData<T>,
}
