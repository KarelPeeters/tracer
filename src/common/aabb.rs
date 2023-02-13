use crate::common::math::{Point3, Transform};
use crate::common::scene::{Object, Shape};

/// Axis-aligned bounding box.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AxisBox {
    pub low: Point3,
    pub high: Point3,
}

const INF: f32 = f32::INFINITY;

impl AxisBox {
    pub fn new(low: Point3, high: Point3) -> Self {
        // TODO add eps padding in here automatically?
        let delta = high - low;
        assert!(delta.x >= 0.0 && delta.y >= 0.0 && delta.z >= 0.0, "low={:?}, high={:?}", low, high);
        Self { low, high }
    }

    pub fn combine(self, other: AxisBox) -> Self {
        AxisBox::new(
            self.low.min(other.low),
            self.high.max(other.high),
        )
    }

    pub fn for_each_corner(self, mut f: impl FnMut(Point3)) {
        f(self.low);
        f(Point3::new(self.high.x, self.low.y, self.low.z));
        f(Point3::new(self.low.x, self.high.y, self.low.z));
        f(Point3::new(self.low.x, self.low.y, self.high.z));
        f(Point3::new(self.high.x, self.high.y, self.low.z));
        f(Point3::new(self.high.x, self.low.y, self.high.z));
        f(Point3::new(self.low.x, self.high.y, self.high.z));
        f(self.high);
    }

    pub fn for_shape(shape: Shape) -> Self {
        match shape {
            Shape::Sphere => AxisBox::new(Point3::new(-1.0, -1.0, -1.0), Point3::new(1.0, 1.0, 1.0)),
            Shape::Plane => AxisBox::new(Point3::new(-INF, -INF, 0.0), Point3::new(INF, INF, 0.0)),
            Shape::Triangle => AxisBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
            Shape::Square => AxisBox::new(Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 1.0, 0.0)),
            Shape::Cylinder => AxisBox::new(Point3::new(-1.0, -INF, -1.0), Point3::new(1.0, INF, 1.0)),
        }
    }

    pub fn for_object(object: &Object) -> Self {
        object.transform * AxisBox::for_shape(object.shape)
    }

    pub fn is_finite(self) -> bool {
        self.low.is_finite() && self.high.is_finite()
    }
}

impl std::ops::Mul<AxisBox> for Transform {
    type Output = AxisBox;

    fn mul(self, rhs: AxisBox) -> Self::Output {
        let mut low = Point3::new(INF, INF, INF);
        let mut high = Point3::new(-INF, -INF, -INF);

        // TODO is there a faster algorithm for this?
        rhs.for_each_corner(|p_orig| {
            let p_trans = self * p_orig;
            low = low.min(p_trans);
            high = high.max(p_trans);
        });

        AxisBox::new(low, high)
    }
}