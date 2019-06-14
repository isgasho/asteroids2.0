use std::mem;
use al::prelude::*;
use astro_lib as al;
use crate::components::Geometry;

pub const EPS: f32 = 1E-3;

pub fn eq(a: f32, b: f32) -> bool {
    (a - b).abs() < EPS
}

// @vlad TODO refactor (it's copy paste from stack overflow)
/// get tangent to circle from point
pub fn get_tangent(circle: Point2, r: f32, point: Point2) -> (Option<Point2>, Option<Point2>) {
    let npoint = (point - circle) / r;
    let xy = npoint.norm_squared();
    if xy - 1.0 <= EPS {
        return (None, None);
    }
    let mut discr = npoint.y * (xy - 1f32).sqrt();
    let tx0 = (npoint.x - discr) / xy;
    let tx1 = (npoint.x + discr) / xy;
    let (yt0, yt1) = if npoint.y != 0f32 {
        (
            circle.y + r * (1f32 - tx0 * npoint.x) / npoint.y,
            circle.y + r * (1f32 - tx1 * npoint.x) / npoint.y,
        )
    } else {
        discr = r * (1f32 - tx0 * tx0).sqrt();
        (circle.y + discr, circle.y - discr)
    };
    let xt0 = circle.x + r * tx0;
    let xt1 = circle.x + r * tx1;
    (Some(Point2::new(xt0, yt0)), Some(Point2::new(xt1, yt1)))
}


/// Polygon for light rendering(just render light on this rctngl)
/// coordinates are in world 3d space
/// Orientation is clockwise
#[derive(Debug)]
pub struct LightningPolygon {
    pub points: Vec<Point2>,
    x_min: f32, 
    y_min: f32, 
    x_max: f32, 
    y_max: f32,
    pub center: Point2, // position of the light
    
}

impl LightningPolygon {
    pub fn new_rectangle(x_min: f32, y_min: f32, x_max: f32, y_max: f32, center: Point2) -> Self {
        // by default we have one big rectangle with no clipping(shadows)
        LightningPolygon {
            points: vec![
                Point2::new(x_min, y_min),
                Point2::new(x_min, y_max),
                Point2::new(x_max, y_max),
                Point2::new(x_max, y_min),
            ],
            x_min, y_min, x_max, y_max,
            center: center,
        }
    }

    pub fn is_border_point(&self, point: Point2) -> bool {
        eq(point.x, self.x_min) || eq(point.x, self.x_max) ||
        eq(point.y, self.y_min) || eq(point.y, self.y_max)
    }

    pub fn get_triangles(&mut self) -> (Vec<Point2>, Vec<u16>) {
        let mut points = vec![];
        points.push(self.center);
        for i in 0..self.points.len() {
            points.push(self.points[i].clone());
        }
        let mut indicies = vec![];
        for i in 1..=self.points.len() {
            indicies.push(0u16);
            indicies.push(i as u16);
            let mut si = i as u16 + 1u16;
            if si == self.points.len() as u16 + 1u16 {
                si = 1u16
            };
            indicies.push(si);
        }
        (points, indicies)
    }

    // slow-ugly-simple version of polygon clipping
    pub fn clip_one(&mut self, geom: Geometry, position: Point2) {
        // PLAN
        // Write clipping with segment
        // then write other primitives with that
        // create rays from center to shape borders
        match geom {
            Geometry::Circle { radius } => {
                let (mut dir1, mut dir2) = match get_tangent(position, radius, self.center) {
                    (Some(p1), Some(p2)) => (
                        Vector2::new(p2.x - self.center.x, p2.y - self.center.y),
                        Vector2::new(p1.x - self.center.x, p1.y - self.center.y),
                    ),
                    _ => return,
                };
                let shape_point1 = self.center + dir1;
                let shape_point2 = self.center + dir2;
                let ray1 = Ray::new(self.center, dir1);
                let ray2 = Ray::new(self.center, dir2);
                // find two points where rays intersect
                let (point1, pid1, point2, pid2) = {
                    // pid1 -- first point of the edge
                    // pid2 -- first point of the edge
                    let mut pi_result1 = None;
                    let mut pi_result2 = None;
                    let mut point_id1 = None;
                    let mut point_id2 = None;
                    for i in 0..self.points.len() {
                        let j = (i + 1) % self.points.len();
                        let p1 = self.points[i];
                        let p2 = self.points[j];
                        let segment = Segment::new(p1, p2);
                        let point_intersect1 =
                            segment.toi_with_ray(&Isometry2::identity(), &ray1, true);
                        let point_intersect2 =
                            segment.toi_with_ray(&Isometry2::identity(), &ray2, true);
                        match point_intersect1 {
                            Some(pi) => {
                                pi_result1 = Some(ray1.point_at(pi));
                                point_id1 = Some(i);
                            }
                            None => (),
                        }
                        match point_intersect2 {
                            Some(pi) => {
                                pi_result2 = Some(ray2.point_at(pi));
                                point_id2 = Some(j);
                            }
                            None => (),
                        }
                    }
                    if pi_result1.is_none() || pi_result2.is_none() {
                        return;
                    };
                    (
                        pi_result1.unwrap(),
                        point_id1.unwrap(),
                        pi_result2.unwrap(),
                        point_id2.unwrap(),
                    )
                };
                if pid1 <= pid2 {
                    self.points.insert(pid1 + 1, point1);
                    self.points.insert(pid1 + 2, shape_point1);
                    self.points.insert(pid1 + 3, shape_point2);
                    self.points.insert((pid2 + 3) % self.points.len(), point2);
                    // remove all points in polygon between them
                    let mut index = -1i32;
                    self.points.retain(|_| {
                        index += 1;
                        !(index > pid1 as i32 + 3 && index < pid2 as i32 + 3)
                    });
                } else {
                    self.points.insert(pid2, point2);
                    self.points.insert(pid1 + 2, point1);
                    self.points.insert(pid1 + 3, shape_point1);
                    self.points.insert(pid1 + 4, shape_point2);
                    let mut index = -1i32;
                    self.points.retain(|_| {
                        index += 1;
                        !(index < pid2 as i32 || index > pid1 as i32 + 4)
                    });
                }
            }
        }
    }
}
