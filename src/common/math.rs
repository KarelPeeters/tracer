use std::cmp::max;
use std::fmt::{Debug, Formatter};
use std::fmt;
use std::ops::{Add, Deref, Div, Mul, Neg, Sub};

pub trait Norm: Div<f32, Output=Self> + Sized + Copy + Debug {
    fn norm_squared(self) -> f32;

    fn norm(self) -> f32 {
        self.norm_squared().sqrt()
    }

    fn try_normalized_and_get(self) -> Option<(Unit<Self>, f32)> {
        let norm = self.norm();
        if norm == 0.0 {
            None
        } else {
            Some((Unit::new_unchecked(self / norm), norm))
        }
    }

    fn normalized_and_get(self) -> (Unit<Self>, f32) {
        self.try_normalized_and_get()
            .unwrap_or_else(|| panic!("norm should be > 0.0 but was {} for {:?}", self.norm(), self))
    }

    fn try_normalized(self) -> Option<Unit<Self>> {
        self.try_normalized_and_get().map(|(u, _)| u)
    }

    fn normalized(self) -> Unit<Self> {
        self.normalized_and_get().0
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub const fn from_slice(array: &[f32; 3]) -> Vec3 {
        Vec3::new(array[0], array[1], array[2])
    }

    pub fn cross(self, other: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn x_axis() -> Unit<Vec3> {
        Unit::new_unchecked(Vec3::new(1.0, 0.0, 0.0))
    }

    pub fn y_axis() -> Unit<Vec3> {
        Unit::new_unchecked(Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn z_axis() -> Unit<Vec3> {
        Unit::new_unchecked(Vec3::new(0.0, 0.0, 1.0))
    }
}

impl Norm for Vec3 {
    fn norm_squared(self) -> f32 {
        self.dot(self)
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub const fn from_slice(array: &[f32; 2]) -> Vec2 {
        Vec2::new(array[0], array[1])
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }
}

impl Norm for Vec2 {
    fn norm_squared(self) -> f32 {
        self.dot(self)
    }
}

// A vector of guaranteed unit length
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Unit<V: Norm> {
    inner: V,
}

impl<V: Norm> Deref for Unit<V> {
    type Target = V;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<V: Norm + Debug> Unit<V> {
    pub fn new_unchecked(inner: V) -> Unit<V> {
        debug_assert!((1.0 - inner.norm_squared()).abs() < 0.00001,
                      "norm_squared should be 1.0 but was {} for {:?}", inner.norm_squared(), inner);
        Unit { inner }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub const fn from_coords(coords: Vec3) -> Self {
        Self::new(coords.x, coords.y, coords.z)
    }

    pub const fn coords(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }

    pub const fn origin() -> Point3 {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn squared_distance_to(self, other: Point3) -> f32 {
        (self - other).norm_squared()
    }

    pub fn distance_to(self, other: Point3) -> f32 {
        (self - other).norm()
    }

    pub fn min(self, other: Point3) -> Point3 {
        Point3::new(self.x.min(other.x), self.y.min(other.y), self.z.min(other.z))
    }

    pub fn max(self, other: Point3) -> Point3 {
        Point3::new(self.x.max(other.x), self.y.max(other.y), self.z.max(other.z))
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Point2 {
    pub coords: Vec2,
}

impl Point2 {
    pub const fn new(x: f32, y: f32) -> Point2 {
        Point2 { coords: Vec2::new(x, y) }
    }

    pub const fn origin() -> Point2 {
        Self::new(0.0, 0.0)
    }

    pub fn squared_distance_to(self, other: Point2) -> f32 {
        (self - other).norm_squared()
    }

    pub fn distance_to(self, other: Point2) -> f32 {
        (self - other).norm()
    }
}

//operator overloading

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Vec3) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Vec3 { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl Div<f32> for Vec3 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Vec3 { x: self.x / rhs, y: self.y / rhs, z: self.z / rhs }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Vec2) -> Self::Output {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Vec2) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        self * -1.0
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Vec2 { x: self.x * rhs, y: self.y * rhs }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Vec2 { x: self.x / rhs, y: self.y / rhs }
    }
}

impl<V: Norm + Neg<Output=V>> Neg for Unit<V> {
    type Output = Unit<V>;
    fn neg(self) -> Self::Output {
        Unit { inner: -self.inner }
    }
}

impl Add<Vec3> for Point3 {
    type Output = Point3;
    fn add(self, rhs: Vec3) -> Self::Output {
        Self::from_coords(self.coords() + rhs)
    }
}

impl Sub<Point3> for Point3 {
    type Output = Vec3;
    fn sub(self, rhs: Point3) -> Self::Output {
        self.coords() - rhs.coords()
    }
}

impl Add<Vec2> for Point2 {
    type Output = Point2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Point2 { coords: self.coords + rhs }
    }
}

impl Sub<Point2> for Point2 {
    type Output = Vec2;
    fn sub(self, rhs: Point2) -> Self::Output {
        self.coords - rhs.coords
    }
}


/// A matrix that behaves like a 4x4 matrix with the last row fixed as [0, 0, 0, 1]
#[derive(Copy, Clone, PartialEq)]
struct Matrix4 {
    rows: [[f32; 4]; 4]
}

impl Default for Matrix4 {
    fn default() -> Self {
        Self::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Debug for Matrix4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            let mut max_size_per_col = [0; 4];
            for c in 0..4 {
                for r in 0..4 {
                    max_size_per_col[c] = max(max_size_per_col[c], format!("{:?}", self.rows[r][c]).len());
                }
            }

            f.write_str("[\n")?;
            for r in 0..4 {
                f.write_str("    ")?;
                for c in 0..4 {
                    if c != 0 {
                        f.write_str(", ")?;
                    }
                    f.write_fmt(format_args!("{:1$?}", self.rows[r][c], max_size_per_col[c]))?;
                }
                f.write_str("\n")?;
            }
            f.write_str("]")
        } else {
            f.debug_struct("Matrix4")
                .field("rows", &self.rows)
                .finish()
        }
    }
}

fn array4_from<T: Default>(mut f: impl FnMut(usize) -> T) -> [T; 4] {
    let mut result: [T; 4] = Default::default();
    for i in 0..4 {
        result[i] = f(i);
    }
    result
}

fn array4x4_from<T: Default>(mut f: impl FnMut(usize, usize) -> T) -> [[T; 4]; 4] {
    array4_from(|r| array4_from(|c| f(r, c)))
}

impl Mul<Matrix4> for Matrix4 {
    type Output = Matrix4;

    fn mul(self, rhs: Matrix4) -> Self::Output {
        Self::new(array4x4_from(|r, c|
            (0..4).map(|i| self.rows[r][i] * rhs.rows[i][c]).sum()
        ))
    }
}

impl Mul<[f32; 4]> for Matrix4 {
    type Output = [f32; 4];

    fn mul(self, rhs: [f32; 4]) -> Self::Output {
        array4_from(|r| (0..4).map(|i| self.rows[r][i] * rhs[i]).sum())
    }
}

impl Matrix4 {
    fn new(rows: [[f32; 4]; 4]) -> Self {
        Self { rows }
    }

    fn transpose(self) -> Self {
        Self::new(array4x4_from(|r, c| self.rows[c][r]))
    }

    fn face_towards(direction: Unit<Vec3>, up: Unit<Vec3>) -> Self {
        let z_axis = -direction;
        let x_axis = up.cross(*z_axis).normalized();
        let y_axis = z_axis.cross(*x_axis).normalized();

        Self::new([
            [x_axis.x, y_axis.x, z_axis.x, 0.0],
            [x_axis.y, y_axis.y, z_axis.y, 0.0],
            [x_axis.z, y_axis.z, z_axis.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn translation(translation: Vec3) -> Self {
        Self::new([
            [1.0, 0.0, 0.0, translation.x],
            [0.0, 1.0, 0.0, translation.y],
            [0.0, 0.0, 1.0, translation.z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn rotation(axis: Unit<Vec3>, angle: Angle) -> Self {
        let Vec3 { x, y, z } = *axis;
        let c = angle.radians.cos();
        let s = angle.radians.sin();

        Self::new([
            [c + x * x * (1.0 - c), x * y * (1.0 - c) - z * s, x * z * (1.0 - c) + y * s, 0.0],
            [y * x * (1.0 - c) + z * s, c + y * y * (1.0 - c), y * z * (1.0 - c) - x * s, 0.0],
            [z * x * (1.0 - c) - y * s, z * y * (1.0 - c) + x * s, c + z * z * (1.0 - c), 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn scaling(scale: f32) -> Self {
        debug_assert!(scale != 0.0);
        Self::new([
            [scale, 0.0, 0.0, 0.0],
            [0.0, scale, 0.0, 0.0],
            [0.0, 0.0, scale, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Transform {
    //TODO also try a translate/quaternion/scale representation and compare performance
    //  size should be way smaller at least, because we wouldn't need to store the inverse
    fwd: Matrix4,
    inv: Matrix4,
}

impl Transform {
    pub fn inv(self) -> Self {
        Self {
            fwd: self.inv,
            inv: self.fwd,
        }
    }

    pub fn inv_transpose_mul(self, rhs: Vec3) -> Vec3 {
        let [x, y, z, _] = self.inv.transpose() * [rhs.x, rhs.y, rhs.z, 0.0];
        Vec3::new(x, y, z)
    }

    pub fn translation(translation: Vec3) -> Self {
        Self {
            fwd: Matrix4::translation(translation),
            inv: Matrix4::translation(-translation),
        }
    }

    pub fn rotation(axis: Unit<Vec3>, angle: Angle) -> Self {
        Self {
            fwd: Matrix4::rotation(axis, angle),
            inv: Matrix4::rotation(axis, -angle),
        }
    }

    pub fn scaling(scale: f32) -> Self {
        Transform {
            fwd: Matrix4::scaling(scale),
            inv: Matrix4::scaling(1.0 / scale),
        }
    }

    /// Translates the origin to `pos` and rotates vectors pointing in the negative Z direction towards `target`
    pub fn look_at(pos: Point3, target: Point3, up: Unit<Vec3>) -> Self {
        let translate = Self::translation(pos.coords());
        let direction = (target - pos).normalized();

        let rotate = Matrix4::face_towards(direction, up);
        let rotate = Self {
            fwd: rotate,
            inv: rotate.transpose(),
        };

        translate * rotate
    }
}


impl Mul<Transform> for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            fwd: self.fwd * rhs.fwd,
            inv: rhs.inv * self.inv,
        }
    }
}

impl Mul<Vec3> for Transform {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Self::Output {
        let [x, y, z, h] = self.fwd * [rhs.x, rhs.y, rhs.z, 0.0];
        debug_assert_eq!(h, 0.0);
        Vec3::new(x, y, z)
    }
}

impl Mul<Point3> for Transform {
    type Output = Point3;

    fn mul(self, rhs: Point3) -> Self::Output {
        let [x, y, z, h] = self.fwd * [rhs.x, rhs.y, rhs.z, 1.0];
        debug_assert_eq!(h, 1.0);
        Point3::new(x, y, z)
    }
}

#[derive(Copy, Clone)]
pub struct Angle {
    pub radians: f32,
}

impl Angle {
    pub fn radians(radians: f32) -> Angle {
        Angle { radians }
    }

    pub fn degrees(degrees: f32) -> Angle {
        Angle::radians(degrees.to_radians())
    }
}

impl Neg for Angle {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Angle { radians: -self.radians }
    }
}

impl Debug for Angle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Angle({} = {}°)", self.radians, self.radians.to_degrees())
    }
}