//NOTE: This library assumes ALL the below structs are using the same
//    number of arguments for areas (Rotors, Bivectors) and dimensions
//    (Vectors).

#[derive(Debug)]
pub struct Rotor<T>{
    pub a: T,
    pub areas: Vec<T>,
}
//Winding order: ..w,z,y,x
#[derive(Debug)]
pub struct BiVector<T> {
    pub areas: Vec<T>,
}
//Winding order: ..w,z,y,x
#[derive(Debug)]
pub struct MultiVector<T> {
    pub grade: usize,
    pub terms: Vec<T>,
}

pub struct Vector<T>{
    pub dimensions: Vec<T>,
}

impl<T: Copy> From<&[T]> for BiVector<T>{
    fn from(s: &[T]) -> Self{
        Self{
            areas: s[..].to_vec(),
        }
    }
}
impl<T: Copy> From<&[T]> for Vector<T>{
    fn from(s: &[T]) -> Self{
        Self{
            dimensions: s[..].to_vec(),
        }
    }
}

impl<T: Copy> From<&[T]> for Rotor<T>{
    fn from(s: &[T]) -> Self{
        Self{
            a: s[0],
            areas: s[1..].to_vec(),
        }
    }
}

pub fn outer<T: Copy + std::ops::Mul<Output=T> + std::ops::Sub<Output=T> + std::fmt::Debug>
                (mut u: Vector<T>, mut v: Vector<T>) -> BiVector<T>{
    let mut areas: Vec<T> = Vec::default();
    let (c, d) = (u.dimensions.as_slice(), v.dimensions.as_mut_slice());
    d.reverse();
    println!("{:?}, {:?}", c, d);
    for ci in 0..(u.dimensions.len() - 1) {
        areas.push(c[ci] * d[ci] - d[ci] * c[ci]);
    }
    areas.as_slice().into()
}

impl<T:  Copy + std::ops::Mul<Output=T> + std::ops::Sub<Output=T>
          + std::ops::Add<Output=T> + std::ops::Div<Output=T>
          + std::ops::Neg<Output=T> + From<f32> + number_traits::Float
          + std::iter::Sum + std::fmt::Display+ std::fmt::Debug>
   Rotor<T>{
    pub fn from_bivector(a: T, bv: BiVector<T>) -> Self{
        Self{
            a,
            areas: bv.areas
        }
    }

    pub fn from_vectors(vFrom: Vector<T>, vTo: Vector<T>) -> Rotor<T>{
        let mut r = Rotor{
            a: vTo.dot(&vFrom) + 1.0.into(),
            areas: outer(vTo, vFrom).areas,
        };
        r.normalize();
        r
    }

    pub fn from_angle_and_axis(angleRadian: T, bvPlane: &BiVector<T>)->Rotor<T>{
        let angle_half = angleRadian / 2.0.into();
        let sina = angle_half.sin();
        Rotor{
            a: angle_half.cos(),
            areas: bvPlane.areas.iter().map(|d| -sina * *d).collect()
        }
    }

    pub fn rotate_by_rotor(self, r: Rotor<T>) -> Rotor<T>{
        let rev = self.reverse();
        self * r * rev
    }

    pub fn reverse(&self) -> Rotor<T>{
        Rotor{
            a: self.a,
            areas: self.areas.iter().map(|d|  -*d ).collect()
        }
    }

    pub fn length_sqrd(&self) -> T{
        let areas_sqrd = self.areas.iter().enumerate().map(
            |(i, d)|{
                *d * *d
            }).sum::<T>();
        areas_sqrd + self.a * self.a
    }

    pub fn length(&self) -> T{ self.length_sqrd().sqrt() }

    pub fn normalize(&mut self){
        let n = self.normal();
        self.a = n.a;
        self.areas = n.areas;
    }

    pub fn normal(&mut self) -> Self{
        let l = self.length();
        let a = self.a / l;
        Rotor{
            a,
            areas: self.areas.iter().map(|d| *d / l).collect()
        }
    }

}

pub fn geo<T>
(u: Vector<T>, v: Vector<T>) -> MultiVector<T> where
    T: Copy + std::ops::Mul<Output=T> + std::ops::Sub<Output=T>
    + std::ops::Add<Output=T> + std::ops::Div<Output=T>
    + std::ops::Neg<Output=T> + From<f32> + number_traits::Float
    + std::iter::Sum + std::fmt::Display + std::fmt::Debug {
    let grade = u.dimensions.len();
    let mut terms = vec![u.dot(&v)];
    terms.append(&mut outer(u, v).areas);
    MultiVector{
        grade,
        terms,
    }
}
impl<T: Copy + std::ops::Mul<Output=T> + std::iter::Sum
      + number_traits::Float + From<f32>>
std::ops::Mul for Rotor<T>{
    type Output = Rotor<T>;

    fn mul(self, rhs: Self) -> Self::Output{
        let u: Vector<T> = (&self.areas[..]).into();
        let v: Vector<T> = (&rhs.areas[..]).into();
        Rotor{
            a: self.a * rhs.a -
                u.dimensions.iter().enumerate().map(|(i, d)|{
                    *d * v.dimensions[i]
                }).sum(),
            areas: u.dimensions.iter().enumerate().map(|(i, d)|{
                v.dimensions.iter().enumerate()
                    .map(|(j, r)| {
                        if i == j {
                            *d * rhs.a + *r + self.a
                        } else{
                            1.0.into()
                        }
                    }).sum()
            }).collect(),
        }
    }
}

impl<T: Copy + std::ops::Mul<Output=T> + std::iter::Sum>
std::ops::Mul for Vector<T>{
    type Output = Vector<T>;

    fn mul(self, rhs: Vector<T>) -> Vector<T>{
        let dimensions: Vec<T> = self.dimensions.iter().enumerate()
            .map(|(i, d)|
                rhs.dimensions.iter().map(|r|
                    *r * *d
                ).sum()).collect();
        Vector{
            dimensions,
        }
    }
}

impl<T:  Copy + std::ops::Mul<Output=T> + std::iter::Sum + number_traits::Float + std::fmt::Display>
Vector<T>{
    pub fn dot(&self, rhs: &Vector<T>) -> T{
        self.dimensions.iter().enumerate().map(
            |(i, d)|{
                *d * rhs.dimensions[i]
        }).sum::<T>()
    }
}

impl<T: Copy + std::ops::Mul<Output=T> + std::ops::Sub<Output=T>
+ std::ops::Add<Output=T> + std::ops::Div<Output=T>
+ std::ops::Neg<Output=T> + From<f32> + number_traits::Float + std::iter::Sum>
Vector<T>{

    pub fn length_sqrd(&self) -> T{
        let areas_sqrd = self.dimensions.iter().enumerate().map(
            |(i, d)|{
                *d * *d
            }).sum::<T>();
        areas_sqrd
    }

    pub fn length(&self) -> T{ self.length_sqrd().sqrt() }

    pub fn normalize(&mut self){
        let n = self.normal();
        self.dimensions = n.dimensions;
    }

    pub fn normal(&mut self) -> Self{
        let l = self.length();
        Vector{
            dimensions: self.dimensions.iter().map(|d| *d / l).collect()
        }
    }
}