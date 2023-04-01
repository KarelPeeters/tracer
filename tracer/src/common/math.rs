use std::cmp::max;
use std::fmt;
use std::fmt::Debug;
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

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
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

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
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

    pub fn middle(self, other: Point3) -> Point3 {
        Point3::new((self.x + other.x) / 2.0, (self.y + other.y) / 2.0, (self.z + other.z) / 2.0)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
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

    pub fn is_finite(self) -> bool {
        self.coords.x.is_finite() && self.coords.y.is_finite()
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

impl Sub<Vec3> for Point3 {
    type Output = Point3;
    fn sub(self, rhs: Vec3) -> Self::Output {
        Self::from_coords(self.coords() - rhs)
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

impl Sub<Vec2> for Point2 {
    type Output = Point2;
    fn sub(self, rhs: Vec2) -> Self::Output {
        Point2 { coords: self.coords - rhs }
    }
}

impl Sub<Point2> for Point2 {
    type Output = Vec2;
    fn sub(self, rhs: Point2) -> Self::Output {
        self.coords - rhs.coords
    }
}

#[derive(Copy, Clone, PartialEq)]
struct Matrix4 {
    rows: [[f32; 4]; 4],
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

    fn translate(translation: Vec3) -> Self {
        Self::new([
            [1.0, 0.0, 0.0, translation.x],
            [0.0, 1.0, 0.0, translation.y],
            [0.0, 0.0, 1.0, translation.z],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn rotate(axis: Unit<Vec3>, angle: Angle) -> Self {
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

    fn scale(scale: f32) -> Self {
        debug_assert!(scale != 0.0);
        Self::new([
            [scale, 0.0, 0.0, 0.0],
            [0.0, scale, 0.0, 0.0],
            [0.0, 0.0, scale, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    fn is_finite(&self) -> bool {
        self.rows.iter().all(|row| row.iter().all(|&x| x.is_finite()))
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

    pub fn translate(translation: Vec3) -> Self {
        Self {
            fwd: Matrix4::translate(translation),
            inv: Matrix4::translate(-translation),
        }
    }

    pub fn rotate(axis: Unit<Vec3>, angle: Angle) -> Self {
        Self {
            fwd: Matrix4::rotate(axis, angle),
            inv: Matrix4::rotate(axis, -angle),
        }
    }

    pub fn scale(scale: f32) -> Self {
        Transform {
            fwd: Matrix4::scale(scale),
            inv: Matrix4::scale(1.0 / scale),
        }
    }

    /// Translates the origin to `pos` and rotates vectors pointing in the negative Z direction towards `target`
    pub fn look_at(pos: Point3, target: Point3, up: Unit<Vec3>) -> Self {
        let dir = (target - pos).normalized();
        Self::look_in_dir(pos, dir, up)
    }

    pub fn look_in_dir(pos: Point3, dir: Unit<Vec3>, up: Unit<Vec3>) -> Self {
        let rotate = Matrix4::face_towards(dir, up);
        let rotate = Self {
            fwd: rotate,
            inv: rotate.transpose(),
        };

        let translate = Self::translate(pos.coords());
        translate * rotate
    }

    /// The transform that maps the unit axis vectors to the given targets.
    /// Does not include a translation.
    pub fn rotate_axes_to(tx: Vec3, ty: Vec3, tz: Vec3) -> Self {
        let fwd = Matrix4::new([
            [tx.x, ty.x, tz.x, 0.0],
            [tx.y, ty.y, tz.y, 0.0],
            [tx.z, ty.z, tz.z, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        let d = tx.x * ty.y * tz.z - tx.x * tz.y * ty.z -
            ty.x * tx.y * tz.z + ty.x *
            tz.y * tx.z + tz.x * tx.y *
            ty.z - tz.x * ty.y * tx.z;

        debug_assert!(d.is_finite() && d != 0.0, "Got invalid determinant {} for mapping vectors {:?}, {:?}, {:?}", d, tx, ty, tz);

        let inv = Matrix4::new([
            [(ty.y * tz.z - tz.y * ty.z) / d, (-ty.x * tz.z + tz.x * ty.z) / d, (ty.x * tz.y - tz.x * ty.y) / d, 0.0],
            [(-tx.y * tz.z + tz.y * tx.z) / d, (tx.x * tz.z - tz.x * tx.z) / d, (-tx.x * tz.y + tz.x * tx.y) / d, 0.0],
            [(tx.y * ty.z - ty.y * tx.z) / d, (-tx.x * ty.z + ty.x * tx.z) / d, (tx.x * ty.y - ty.x * tx.y) / d, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        Transform { fwd, inv }
    }

    pub fn is_finite(&self) -> bool {
        self.fwd.is_finite() && self.inv.is_finite()
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Angle({} = {}°)", self.radians, self.radians.to_degrees())
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
    X,
    Y,
    Z,
}

impl Axis3 {
    pub const ALL: [Axis3; 3] = [Axis3::X, Axis3::Y, Axis3::Z];
}

pub trait Axis3Owner {
    fn get(self, axis: Axis3) -> f32;
}

impl Axis3Owner for Point3 {
    fn get(self, axis: Axis3) -> f32 {
        match axis {
            Axis3::X => self.x,
            Axis3::Y => self.y,
            Axis3::Z => self.z,
        }
    }
}

impl Axis3Owner for Vec3 {
    fn get(self, axis: Axis3) -> f32 {
        match axis {
            Axis3::X => self.x,
            Axis3::Y => self.y,
            Axis3::Z => self.z,
        }
    }
}

pub fn lerp(t: f32, x: f32, y: f32) -> f32 {
    t * x + (1.0 - t) * y
}

#[cfg(test)]
mod test {
    use crate::common::math::{Point3, Transform, Vec3};

    fn assert_close_vec3(left: Vec3, right: Vec3) {
        let delta = left - right;
        let max_delta = delta.x.max(delta.y).max(delta.z);
        assert!(
            left.is_finite() && right.is_finite() && max_delta < 0.0001,
            "Expected close, finite values, got {left:?} and {right:?}"
        );
    }

    fn assert_close_point3(left: Point3, right: Point3) {
        assert_close_vec3(left - Point3::origin(), right - Point3::origin());
    }

    #[test]
    fn rotate_axes_to() {
        let tx = Vec3::new(1.0, 2.0, 3.0);
        let ty = Vec3::new(4.0, 5.0, 6.0);
        let tz = Vec3::new(2.0, 4.0, 8.0);

        let trans = Transform::rotate_axes_to(tx, ty, tz);

        println!("{:?}", trans);

        assert_close_vec3(tx, trans * *Vec3::x_axis());
        assert_close_vec3(ty, trans * *Vec3::y_axis());
        assert_close_vec3(tz, trans * *Vec3::z_axis());
        assert_close_point3(Point3::origin(), trans * Point3::origin());

        assert_close_vec3(*Vec3::x_axis(), trans.inv() * tx);
        assert_close_vec3(*Vec3::y_axis(), trans.inv() * ty);
        assert_close_vec3(*Vec3::z_axis(), trans.inv() * tz);
        assert_close_point3(Point3::origin(), trans.inv() * Point3::origin());

        let unit = trans.fwd * trans.inv;
        println!("{:?}", unit);
    }
}