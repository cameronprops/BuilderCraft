//! Headless alpha subsets of interactive modeling transforms.
//!
//! Every operation is bounded, pure, and reusable by direct CAD and OrbWeaver.
//! These are intentionally not full Rhino PushPull/FlowAlongSrf/Project parity:
//! PushPull creates a closed prism from an isolated planar convex quad, not a
//! Boolean edit to existing BRep; Flow maps via a planar base parallelogram to
//! a bilinear target patch; Project intersects an infinite line with a plane.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_validate, polyline_divide_count};
use cadcraft_geom::Vec3;

const MAX_SEED: usize = 1024;
const MAX_INSTANCES: usize = 256;
const MAX_TOTAL: usize = 65_536;
const LIMIT: f64 = 1e12;

fn valid(p: Vec3) -> bool {
    p.is_finite() && p.x.abs().max(p.y.abs()).max(p.z.abs()) <= LIMIT
}
fn check(p: Vec3) -> Result<Vec3> {
    if valid(p) { Ok(p) } else { Err(KernelError::Invalid("nonfinite/oversized transform coordinate")) }
}
fn unit(v: Vec3) -> Result<Vec3> {
    check(v)?;
    let n = v.len();
    if !n.is_finite() || n < 1e-10 { return Err(KernelError::Invalid("zero or degenerate axis")); }
    Ok(v * (1. / n))
}
fn seed_check(seed: &[Vec3], count: usize) -> Result<()> {
    if seed.is_empty() || count == 0 { return Err(KernelError::Invalid("array requires geometry and copies")); }
    if seed.len() > MAX_SEED || count > MAX_INSTANCES || seed.len().checked_mul(count).ok_or(KernelError::Budget)? > MAX_TOTAL {
        return Err(KernelError::Budget);
    }
    for &point in seed { check(point)?; }
    Ok(())
}
fn translated(seed: &[Vec3], delta: Vec3) -> Result<Vec<Vec3>> {
    seed.iter().map(|p| check(*p + delta)).collect()
}

/// Complete geometry copies as separate point sequences; first copy is original.
pub fn array_linear(seed: &[Vec3], step: Vec3, count: usize) -> Result<Vec<Vec<Vec3>>> {
    seed_check(seed, count)?;
    check(step)?;
    (0..count).map(|i| translated(seed, step * (i as f64))).collect()
}

/// Row-major rectangular / volumetric array. No object copies are flattened.
pub fn array_rectangular(seed: &[Vec3], x_step: Vec3, y_step: Vec3, z_step: Vec3, nx: usize, ny: usize, nz: usize) -> Result<Vec<Vec<Vec3>>> {
    let count = nx.checked_mul(ny).and_then(|n| n.checked_mul(nz)).ok_or(KernelError::Budget)?;
    seed_check(seed, count)?;
    for step in [x_step, y_step, z_step] { check(step)?; }
    let mut out = Vec::new();
    out.try_reserve_exact(count).map_err(|_| KernelError::Budget)?;
    for z in 0..nz { for y in 0..ny { for x in 0..nx {
        out.push(translated(seed, x_step * x as f64 + y_step * y as f64 + z_step * z as f64)?);
    }}}
    Ok(out)
}

/// Right-hand rotations. A complete 360 degree sweep omits duplicate endpoint.
pub fn array_polar(seed: &[Vec3], center: Vec3, axis: Vec3, sweep_degrees: f64, count: usize) -> Result<Vec<Vec<Vec3>>> {
    seed_check(seed, count)?;
    check(center)?;
    let axis = unit(axis)?;
    if !sweep_degrees.is_finite() || sweep_degrees.abs() > 3600.0 { return Err(KernelError::Invalid("polar sweep")); }
    let complete = (sweep_degrees.abs() - 360.0).abs() < 1e-9;
    let divisor = if complete { count } else { count.saturating_sub(1).max(1) };
    let mut copies = Vec::new();
    copies.try_reserve_exact(count).map_err(|_| KernelError::Budget)?;
    for i in 0..count {
        let angle = (sweep_degrees / divisor as f64 * i as f64).to_radians();
        let (s, c) = angle.sin_cos();
        let copy = seed.iter().map(|p| {
            let v = *p - center;
            check(center + v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1.0-c)))
        }).collect::<Result<Vec<_>>>()?;
        copies.push(copy);
    }
    Ok(copies)
}

/// Pointwise path placements by arc length. No tangent-based orientation yet.
pub fn array_path(seed: &[Vec3], path: &[Vec3], count: usize) -> Result<Vec<Vec<Vec3>>> {
    seed_check(seed, count)?;
    if path.len() < 2 || path.len() > MAX_SEED { return Err(KernelError::Invalid("array path")); }
    for &p in path { check(p)?; }
    let samples = polyline_divide_count(path, count.saturating_sub(1).max(1))?;
    if samples.len() != count && count != 1 { return Err(KernelError::Invalid("path sample count")); }
    let mut copies = Vec::new();
    for p in samples.iter().take(count) { copies.push(translated(seed, *p - path[0])?); }
    Ok(copies)
}

/// Infinite directed-line intersection with a plane. No mesh/surface trimming.
pub fn project_to_plane(points: &[Vec3], origin: Vec3, normal: Vec3, direction: Vec3) -> Result<Vec<Vec3>> {
    if points.is_empty() || points.len() > MAX_TOTAL { return Err(KernelError::Budget); }
    check(origin)?;
    let normal = unit(normal)?;
    let direction = unit(direction)?;
    let denominator = normal.dot(direction);
    if denominator.abs() < 1e-9 { return Err(KernelError::Invalid("projection direction parallel to plane")); }
    points.iter().map(|p| {
        check(*p)?;
        let distance = normal.dot(origin - *p) / denominator;
        check(*p + direction * distance)
    }).collect()
}
fn patch(base: &[Vec3]) -> Result<[Vec3;4]> {
    if base.len() != 4 { return Err(KernelError::Invalid("surface patch needs four corners")); }
    Ok([check(base[0])?, check(base[1])?, check(base[2])?, check(base[3])?])
}
fn basis(patch: [Vec3;4]) -> Result<(Vec3, Vec3, Vec3, f64)> {
    let u = patch[1] - patch[0];
    let v = patch[3] - patch[0];
    let cross = u.cross(v);
    let area = cross.len();
    if area < 1e-9 || !area.is_finite() { return Err(KernelError::Invalid("degenerate surface patch")); }
    Ok((u, v, cross * (1.0 / area), area))
}
/// Map points on a planar parallelogram to a bilinear (possibly warped) patch.
/// Preserves the base-normal offset measured in model units.
pub fn flow_along_patch(points: &[Vec3], base: &[Vec3], target: &[Vec3]) -> Result<Vec<Vec3>> {
    if points.is_empty() || points.len() > MAX_TOTAL { return Err(KernelError::Budget); }
    let b = patch(base)?;
    let t = patch(target)?;
    let (bu, bv, bn, _) = basis(b)?;
    let _ = basis(t)?;
    let area2 = bu.cross(bv).dot(bu.cross(bv));
    let a = bu.dot(bu);
    let d = bu.dot(bv);
    let c = bv.dot(bv);
    let determinant = a * c - d * d;
    if !determinant.is_finite() || determinant < 1e-20 || determinant / (a*c) < 1e-12 {
        return Err(KernelError::Invalid("near-singular base surface"));
    }
    let closing_error = (b[2] - (b[1] + b[3] - b[0])).len();
    if closing_error > (bu.len()+bv.len())*1e-8 { return Err(KernelError::Invalid("base patch must be parallelogram")); }
    let _ = area2;
    points.iter().map(|p| {
        check(*p)?;
        let delta = *p-b[0];
        let x = delta.dot(bu);
        let y = delta.dot(bv);
        let u = (x*c - y*d)/determinant;
        let v = (y*a - x*d)/determinant;
        if !u.is_finite() || !v.is_finite() || !(-1e-8..=1.0+1e-8).contains(&u) || !(-1e-8..=1.0+1e-8).contains(&v) {
            return Err(KernelError::Invalid("flow point outside base patch"));
        }
        let u = u.clamp(0.,1.);
        let v = v.clamp(0.,1.);
        let surface = t[0]*((1.-u)*(1.-v))+t[1]*(u*(1.-v))+t[2]*(u*v)+t[3]*((1.-u)*v);
        let du = (t[1]-t[0])*(1.-v)+(t[2]-t[3])*v;
        let dv = (t[3]-t[0])*(1.-u)+(t[2]-t[1])*u;
        let normal = unit(du.cross(dv))?;
        check(surface + normal * delta.dot(bn))
    }).collect()
}

/// Isolated convex, coplanar quad -> six-face closed polygon prism.
/// No Booleans, no face-picking/extend mode, and no existing solid edit.
pub fn pushpull_quad(face: &[Vec3], distance: f64) -> Result<PolygonMesh> {
    let c = patch(face)?;
    if !distance.is_finite() || distance.abs() < 1e-9 || distance.abs() > 1e9 {
        return Err(KernelError::Invalid("pushpull distance"));
    }
    let (u,v,n,_) = basis(c)?;
    if (c[2]-c[0]).dot(n).abs() > 1e-8*(u.len()+v.len()) {
        return Err(KernelError::Invalid("pushpull quad must be planar"));
    }
    let mut vertices=c.to_vec();
    for p in c { vertices.push(check(p+n*distance)?); }
    let mut faces=Vec::new();
    if distance>0. {
        faces.push(PolygonFace::Quad([3,2,1,0]));
        faces.push(PolygonFace::Quad([4,5,6,7]));
    } else {
        faces.push(PolygonFace::Quad([0,1,2,3]));
        faces.push(PolygonFace::Quad([7,6,5,4]));
    }
    for i in 0..4u32 {
        let j=(i+1)%4;
        faces.push(if distance>0. {
            PolygonFace::Quad([i,j,j+4,i+4])
        } else {
            PolygonFace::Quad([j,i,i+4,j+4])
        });
    }
    let mesh=PolygonMesh{vertices,faces};
    polygon_mesh_validate(&mesh)?;
    Ok(mesh)
}

/// Extrude a picked, connected triangle/quad face of an existing *closed*
/// orientable polygon mesh. The original cap is replaced by a translated cap
/// and a ring of quad walls. Native neighboring polygons retain their IDs.
/// Caller owns transaction, object identity, undo and revision enforcement.
///
/// This is topology-aware face extrusion, not arbitrary BRep boolean pushpull.
/// Tessellated convexity, collision and watertightness checks remain mandatory.
pub fn pushpull_mesh_face(mesh: &PolygonMesh, face_index: usize, distance: f64) -> Result<PolygonMesh> {
    polygon_mesh_validate(mesh)?;
    if !distance.is_finite() || distance.abs() < 1e-9 || distance.abs() > 1e9 {
        return Err(KernelError::Invalid("pushpull face distance"));
    }
    let face = mesh.faces.get(face_index).ok_or(KernelError::Invalid("pushpull face index"))?;
    let corners = face.indices();
    let first = mesh.vertices[corners[0] as usize];
    let normal = unit((mesh.vertices[corners[1] as usize]-first).cross(mesh.vertices[corners[2] as usize]-first))?;
    let base_length = (mesh.vertices[corners[1] as usize]-first).len().max((mesh.vertices[corners[2] as usize]-first).len());
    for &id in &corners {
        let p = mesh.vertices[id as usize];
        if (p-first).dot(normal).abs() > 1e-8*base_length {
            return Err(KernelError::Invalid("pushpull selected face must be planar"));
        }
    }
    let topology=crate::polygon_mesh_topology(mesh)?;
    if !topology.boundary_edges.is_empty() || !topology.non_manifold_edges.is_empty() || !topology.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("pushpull requires a closed manifold mesh"));
    }
    if topology.face_neighbors.get(face_index).is_none_or(|neighbors| neighbors.iter().any(Option::is_none)) {
        return Err(KernelError::Invalid("pushpull selected face must be connected"));
    }
    if mesh.vertices.len()+corners.len() > 100_000 || mesh.faces.len()+corners.len() > 100_000 { return Err(KernelError::Budget); }
    // Reject through-body collapse of the swept cap. This conservative
    // restriction is not an exact self-intersection solver.
    if distance < 0.0 {
        let minimum_inside = mesh.vertices.iter().enumerate().filter(|(i,_)| !corners.iter().any(|id| *id as usize == *i))
            .map(|(_,v)| (first-*v).dot(normal)).filter(|d| *d > 1e-9).fold(f64::INFINITY, f64::min);
        if minimum_inside.is_finite() && -distance >= minimum_inside - 1e-8 {
            return Err(KernelError::Invalid("pushpull would cross the opposite mesh"));
        }
    }
    let base=u32::try_from(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    let mut next=mesh.clone();
    for id in &corners { next.vertices.push(check(mesh.vertices[*id as usize]+normal*distance)?); }
    let new: Vec<u32> = (0..corners.len()).map(|i| base+u32::try_from(i).unwrap_or(0)).collect();
    next.faces[face_index]=match new.as_slice() {
        [a,b,c] => PolygonFace::Triangle([*a,*b,*c]),
        [a,b,c,d] => PolygonFace::Quad([*a,*b,*c,*d]),
        _ => return Err(KernelError::Invalid("face must have 3 or 4 corners")),
    };
    for i in 0..corners.len() {
        let j=(i+1)%corners.len();
        next.faces.push(PolygonFace::Quad([corners[i],corners[j],new[j],new[i]]));
    }
    polygon_mesh_validate(&next)?;
    let after=crate::polygon_mesh_topology(&next)?;
    if !after.boundary_edges.is_empty() || !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("pushpull result is not a watertight orientable mesh"));
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x:f64,y:f64,z:f64)->Vec3{Vec3::new(x,y,z)}
    fn square()->Vec<Vec3>{vec![p(0.,0.,0.),p(2.,0.,0.),p(2.,2.,0.),p(0.,2.,0.)]}
    #[test]
    fn arrays_keep_geometry_and_polar_endpoints() {
        let seed=vec![p(2.,0.,0.),p(3.,0.,0.)];
        let a=array_linear(&seed,p(5.,0.,0.),3).unwrap();
        assert_eq!(a[2],vec![p(12.,0.,0.),p(13.,0.,0.)]);
        let b=array_rectangular(&seed,p(2.,0.,0.),p(0.,3.,0.),p(0.,0.,4.),2,2,2).unwrap();
        assert_eq!(b.len(),8);
        assert_eq!(b[7][0],p(4.,3.,4.));
        let r=array_polar(&[p(1.,0.,0.)],Vec3::ZERO,Vec3::Z,360.,4).unwrap();
        assert!((r[1][0]-p(0.,1.,0.)).len()<1e-9);
        assert!((r[3][0]-p(0.,-1.,0.)).len()<1e-9);
    }
    #[test]
    fn project_and_flow_boundaries() {
        let projected=project_to_plane(&[p(2.,3.,10.)],Vec3::ZERO,Vec3::Z,p(0.,0.,-1.)).unwrap();
        assert_eq!(projected,[p(2.,3.,0.)]);
        assert!(project_to_plane(&[p(0.,0.,1.)],Vec3::ZERO,Vec3::Z,p(1.,0.,0.)).is_err());
        let base=square();
        let target=vec![p(0.,0.,5.),p(2.,0.,5.),p(2.,2.,5.),p(0.,2.,5.)];
        let flow=flow_along_patch(&[p(1.,1.,0.5)],&base,&target).unwrap();
        assert!((flow[0]-p(1.,1.,5.5)).len()<1e-9);
        assert!(flow_along_patch(&[p(9.,9.,0.)],&base,&target).is_err());
    }
    #[test]
    fn pushpull_returns_closed_quad_prism_and_rejects_bad_cases() {
        let m=pushpull_quad(&square(),3.).unwrap();
        assert_eq!(m.vertices.len(),8);
        assert_eq!(m.faces.len(),6);
        assert_eq!(m.vertices[4],p(0.,0.,3.));
        assert_eq!(pushpull_quad(&square(),-3.).unwrap().faces.len(),6);
        assert!(pushpull_quad(&square(),0.).is_err());
        assert!(pushpull_quad(&[p(0.,0.,0.),p(1.,0.,0.),p(1.,1.,2.),p(0.,1.,0.)],1.).is_err());
    }
    #[test]
    fn connected_face_pushpull_extends_closed_solid_and_preserves_original() {
        let cube=pushpull_quad(&square(),2.).unwrap();
        let old=cube.clone();
        let extended=pushpull_mesh_face(&cube,1,3.).unwrap();
        assert_eq!(extended.vertices.len(),12);
        assert_eq!(extended.faces.len(),10);
        assert_eq!(cube,old);
        let t=crate::polygon_mesh_topology(&extended).unwrap();
        assert!(t.boundary_edges.is_empty());
        assert!(t.non_manifold_edges.is_empty());
        assert!(t.inconsistent_winding_edges.is_empty());
        assert!(pushpull_mesh_face(&cube,99,1.).is_err());
        assert!(pushpull_mesh_face(&cube,1,0.).is_err());
        assert!(pushpull_mesh_face(&cube,1,-3.).is_err());
    }
    #[test]
    fn allocations_reject_excess() {
        assert!(array_linear(&[Vec3::ZERO],Vec3::Z,257).is_err());
        assert!(array_rectangular(&[Vec3::ZERO],Vec3::Z,Vec3::Z,Vec3::Z,usize::MAX,2,1).is_err());
    }
}
