/// Can be specified as 0xAARRGGBB
#[derive(Clone, Copy, Debug, Hash, PartialEq, PartialOrd)]
pub struct Color(pub u32);

const GAMMA: f32 = 2.2;

impl Color {
    pub const BLACK_TRANSPARENT: Self = Self(0x0);
    pub const BLACK_OPAQUE: Self = Self(0xFF000000);
    pub const RED: Self = Self(0xFFFF0000);
    pub const GREEN: Self = Self(0xFF00FF00);
    pub const BLUE: Self = Self(0xFF0000FF);

    fn import(value: f32) -> u32 {
        (value.clamp(0.0, 1.0) * 255.0) as u32
    }

    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self(
            (Self::import(alpha) << 24)
                | (Self::import(red) << 16)
                | (Self::import(green) << 8)
                | Self::import(blue),
        )
    }

    pub fn from_rgba(d: [f32; 4]) -> Self {
        Self::new(d[0], d[1], d[2], d[3])
    }
    pub fn from_rgb_alpha(d: [f32; 3], alpha: f32) -> Self {
        Self::new(d[0], d[1], d[2], alpha)
    }

    fn export(self, index: u32) -> f32 {
        ((self.0 >> (index << 3)) & 0xFF) as f32 / 255.0
    }
    pub fn red(self) -> f32 {
        self.export(2)
    }
    pub fn green(self) -> f32 {
        self.export(1)
    }
    pub fn blue(self) -> f32 {
        self.export(0)
    }
    pub fn alpha(self) -> f32 {
        self.export(3)
    }
    pub fn into_vec4(self) -> [f32; 4] {
        [self.red(), self.green(), self.blue(), self.alpha()]
    }
    pub fn into_vec4_gamma(self) -> [f32; 4] {
        [
            self.red().powf(GAMMA),
            self.green().powf(GAMMA),
            self.blue().powf(GAMMA),
            self.alpha().powf(GAMMA),
        ]
    }
}

impl From<Color> for wgpu::Color {
    fn from(c: Color) -> Self {
        Self {
            r: c.red() as f64,
            g: c.green() as f64,
            b: c.blue() as f64,
            a: c.alpha() as f64,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK_OPAQUE
    }
}
