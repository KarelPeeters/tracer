use crate::cs;

pub enum Material {
    Fixed {
        color: [f32; 3]
    },
    Opaque {
        color: [f32; 3],
        mirror: f32,
        diffuse: f32,
    },
    Transparent {
        color: [f32; 3],
        refract_ratio: f32,

        mirror: f32,
        diffuse: f32,
        transparent: f32,
    },
}

impl Material {
    pub fn as_cs_ty(&self) -> cs::ty::Material {
        match self {
            Material::Fixed { color } => cs::ty::Material {
                color: *color,
                fixedColor: true as u32,
                refractRatio: 0.0,

                keyDiffuse: 1.0,
                keyTransparent: 1.0,

                _dummy0: Default::default(),
            },
            Material::Opaque { color, mirror, diffuse } => {
                let total = mirror + diffuse;
                cs::ty::Material {
                    color: *color,
                    fixedColor: false as u32,
                    refractRatio: 0.0,

                    keyDiffuse: diffuse / total,
                    keyTransparent: 1.0,

                    _dummy0: Default::default(),
                }
            }
            Material::Transparent { color, refract_ratio, mirror, diffuse, transparent } => {
                let total = mirror + diffuse + transparent;
                cs::ty::Material {
                    color: *color,
                    fixedColor: false as u32,
                    refractRatio: *refract_ratio,

                    keyDiffuse: diffuse / total,
                    keyTransparent: 1.0 - transparent / total,

                    _dummy0: Default::default(),
                }
            },
        }
    }
}