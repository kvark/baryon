#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeReference(u32);

#[derive(Debug, PartialEq)]
struct Space {
    position: mint::Vector3<f32>,
    scale: f32,
    orientation: mint::Quaternion<f32>,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            position: mint::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
            scale: 1.0,
            orientation: mint::Quaternion { s: 1.0, v: mint::Vector3 { x: 0.0, y: 0.0, z: 0.0 }},
        }
    }
}

#[derive(Default, Debug, PartialEq)]
struct Node {
    parent: NodeReference,
    local: Space,
}

pub struct Scene {
    nodes: Vec<Node>,
}

impl Scene {
    fn add_node(&mut self, node: Node) -> NodeReference {
        if node.local == Space::default() {
            node.parent
        } else {
            let index = self.nodes.len();
            self.nodes.push(node);
            NodeReference(index as u32)
        }
    }

    pub fn new(&mut self) -> ObjectBuilder<()> {
        ObjectBuilder {
            scene: self,
            node: Node::default(),
            kind: (),
        }
    }
}

pub struct ObjectBuilder<'a, T> {
    scene: &'a mut Scene,
    node: Node,
    kind: T,
}

impl<T> ObjectBuilder<'_, T> {
    pub fn position(mut self, position: mint::Vector3<f32>) -> Self {
        self.node.local.position = position;
        self
    }
}

impl ObjectBuilder<'_, ()> {
    pub fn done(self) -> NodeReference {
        self.scene.add_node(self.node)
    }
}
