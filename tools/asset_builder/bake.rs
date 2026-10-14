use crate::{convert::Catalog, core::*, geometry::{self, Surface}, pack::Pack, texture};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

const GUTTER: usize = 8;

#[path = "bake_gpu.rs"]
mod gpu;

struct Input {
    width: usize,
    height: usize,
    sampler: u8,
    pixels: Vec<[f32; 4]>,
}

impl Input {
    fn read(image: &texture::Image, sampler: u8) -> Result<Self> {
        if image.faces != 1 || image.depth != 1 { return Err("Layered material input is not a 2D image".into()); }
        Ok(Self { width:image.width, height:image.height, sampler, pixels:texture::pixels(image, 0)? })
    }

    fn sample(&self, uv: [f32; 2]) -> [f32; 4] {
        texture::sample(&self.pixels,self.width,self.height,uv,self.sampler)
    }
}

struct ChartTriangle {
    surface: usize,
    first: usize,
    points: [[f64;2];3],
    metric: [f64;3],
    copies: Vec<(usize,usize,[usize;3])>,
}

struct Chart {
    triangles: Vec<ChartTriangle>,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
}

struct AtlasPage {
    charts: Vec<Chart>,
    rows: Vec<(usize,usize,usize)>,
    width: usize,
    height: usize,
}

fn place_pages(mut charts: Vec<Chart>) -> Result<Vec<AtlasPage>> {
    for chart in &mut charts {
        let limit = (8192-2*GUTTER) as f64;
        if chart.width as f64 <= limit && chart.height as f64 <= limit { continue; }
        let points: Vec<_> = chart.triangles.iter().flat_map(|triangle| triangle.points).collect();
        let quarter = std::f64::consts::FRAC_PI_2;
        let mut angles = vec![0.,quarter];
        for (index,a) in points.iter().enumerate() { for b in &points[index+1..] {
            let (x,y) = (b[0]-a[0],b[1]-a[1]);
            let length = x.hypot(y);
            if length <= limit { continue; }
            let phase = y.atan2(x);
            let boundary = (limit/length).acos();
            for angle in [boundary-phase,-boundary-phase,quarter+boundary-phase,quarter-boundary-phase] {
                angles.push(angle.rem_euclid(quarter));
            }
        }}
        angles.sort_by(f64::total_cmp);
        angles.dedup();
        let candidates: Vec<_> = angles.windows(2).map(|pair| (pair[0]+pair[1])*0.5).chain(angles.iter().copied()).collect();
        for angle in candidates {
            let (sin,cos) = angle.sin_cos();
            let rotate = |point: [f64;2]| [point[0]*cos-point[1]*sin,point[0]*sin+point[1]*cos];
            let lower: [f64;2] = std::array::from_fn(|axis| points.iter().map(|&point| rotate(point)[axis]).fold(f64::INFINITY,f64::min));
            let upper: [f64;2] = std::array::from_fn(|axis| points.iter().map(|&point| rotate(point)[axis]).fold(f64::NEG_INFINITY,f64::max));
            let width = (upper[0]-lower[0]).ceil() as usize;
            let height = (upper[1]-lower[1]).ceil() as usize;
            if width+2*GUTTER > 8192 || height+2*GUTTER > 8192 { continue; }
            for triangle in &mut chart.triangles {
                for point in &mut triangle.points { *point=std::array::from_fn(|axis| rotate(*point)[axis]-lower[axis]); }
            }
            println!("Rotated oversized source chart {}x{} into {width}x{height} without changing source triangles or directional footprints",chart.width,chart.height);
            chart.width=width.max(2); chart.height=height.max(2);
            break;
        }
    }
    charts.sort_by_key(|chart| (std::cmp::Reverse(chart.height),std::cmp::Reverse(chart.width)));
    let mut pages = Vec::<AtlasPage>::new();
    for mut chart in charts {
        let (width,height) = (chart.width+2*GUTTER,chart.height+2*GUTTER);
        if width > 8192 || height > 8192 {
            let triangle = &chart.triangles[0];
            return Err(format!("Bake chart requires {width}x{height}, beyond an 8192 page: source surface {}, triangle {}; source-preserving subdivision is required",triangle.surface,triangle.first/3).into());
        }
        let surface = chart.triangles[0].surface;
        let mut best = None;
        for (page_index,page) in pages.iter().enumerate() {
            let shared = page.charts.iter().any(|chart| chart.triangles.iter().any(|triangle| triangle.surface == surface));
            for (row_index,&(_,row_height,x)) in page.rows.iter().enumerate() {
                if height <= row_height && x+width <= 8192 {
                    let score = ((row_height-height)*width,!shared,8192-x-width,page_index,row_index);
                    if best.is_none_or(|current| score < current) { best=Some(score); }
                }
            }
        }
        let (page_index,row_index) = if let Some((_,_,_,page,row)) = best { (page,row) } else {
            let available = pages.iter().enumerate().filter_map(|(index,page)| {
                let y = page.rows.last().map_or(0,|&(y,height,_)| y+height);
                let shared = page.charts.iter().any(|chart| chart.triangles.iter().any(|triangle| triangle.surface == surface));
                (y+height <= 8192).then(|| (!shared,8192-y-height,index,y))
            }).min();
            let (page,y) = if let Some((_,_,page,y)) = available { (page,y) } else {
                pages.push(AtlasPage {charts:Vec::new(),rows:Vec::new(),width:0,height:0});
                (pages.len()-1,0)
            };
            pages[page].rows.push((y,height,0));
            (page,pages[page].rows.len()-1)
        };
        let page = &mut pages[page_index];
        let (y,_,x) = &mut page.rows[row_index];
        chart.x=*x; chart.y=*y;
        *x+=width;
        page.width=page.width.max(*x);
        page.height=page.height.max(*y+height);
        page.charts.push(chart);
    }
    for page in &mut pages {
        page.width=page.width.max(4).next_power_of_two();
        page.height=page.height.max(4).next_power_of_two();
        page.rows.clear();
        for chart in &page.charts {
            if chart.x+chart.width+2*GUTTER > page.width || chart.y+chart.height+2*GUTTER > page.height {
                return Err("Atlas chart placement exceeds its page".into());
            }
        }
    }
    Ok(pages)
}

fn overlaps(left: [[f64;2];3], right: [[f64;2];3]) -> bool {
    for triangle in [left,right] {
        for edge in 0..3 {
            let axis = [triangle[(edge+1)%3][1]-triangle[edge][1],triangle[edge][0]-triangle[(edge+1)%3][0]];
            let range = |points: [[f64;2];3]| points.map(|point| point[0]*axis[0]+point[1]*axis[1]);
            let a = range(left); let b = range(right);
            if a.into_iter().fold(f64::NEG_INFINITY,f64::max) <= b.into_iter().fold(f64::INFINITY,f64::min)
                || b.into_iter().fold(f64::NEG_INFINITY,f64::max) <= a.into_iter().fold(f64::INFINITY,f64::min) { return false; }
        }
    }
    true
}

fn triangle_weights(vertices: [[f64;2];3], point: [f64;2]) -> Result<([f64;3],f64)> {
    let a = [vertices[1][0]-vertices[0][0],vertices[1][1]-vertices[0][1]];
    let b = [vertices[2][0]-vertices[0][0],vertices[2][1]-vertices[0][1]];
    let p = [point[0]-vertices[0][0],point[1]-vertices[0][1]];
    let determinant = a[0]*b[1]-a[1]*b[0];
    if determinant == 0. || !determinant.is_finite() { return Err("Degenerate bake triangle".into()); }
    let s = (p[0]*b[1]-p[1]*b[0])/determinant;
    let t = (a[0]*p[1]-a[1]*p[0])/determinant;
    let weights = [1.-s-t,s,t];
    if weights.iter().all(|&value| value >= 0.) { return Ok((weights,0.)); }
    let mut best = ([0.;3],f64::INFINITY);
    for edge in 0..3 {
        let next = (edge+1)%3;
        let d = [vertices[next][0]-vertices[edge][0],vertices[next][1]-vertices[edge][1]];
        let q = [point[0]-vertices[edge][0],point[1]-vertices[edge][1]];
        let length = d[0]*d[0]+d[1]*d[1];
        let along = if length > 0. { ((q[0]*d[0]+q[1]*d[1])/length).clamp(0.,1.) } else { 0. };
        let distance = (q[0]-along*d[0]).powi(2)+(q[1]-along*d[1]).powi(2);
        if distance < best.1 {
            let mut weights = [0.;3];
            weights[edge]=1.-along; weights[next]=along;
            best=(weights,distance);
        }
    }
    Ok(best)
}

fn merge_islands(charts: Vec<Chart>, surfaces: &[Surface]) -> Result<Vec<Chart>> {
    let source_triangles: usize = charts.iter().flat_map(|chart| &chart.triangles).map(|triangle| triangle.copies.len()).sum();
    let mut inputs = Vec::new();
    let mut edges = BTreeMap::<(Vec<u32>,Vec<u32>),Vec<usize>>::new();
    for (index,chart) in charts.iter().enumerate() {
        let triangle = &chart.triangles[0];
        let surface = &surfaces[triangle.surface];
        let uv: [[f64;2];3] = std::array::from_fn(|i| surface.uv[triangle.first+i].map(f64::from));
        let a = [uv[1][0]-uv[0][0],uv[1][1]-uv[0][1]];
        let b = [uv[2][0]-uv[0][0],uv[2][1]-uv[0][1]];
        let determinant = a[0]*b[1]-a[1]*b[0];
        let size = a.into_iter().chain(b).map(f64::abs).fold(0.,f64::max);
        if determinant.abs() <= size*size*1e-12 { inputs.push(None); continue; }
        let x = [b[1]/determinant,-a[1]/determinant];
        let y = [-b[0]/determinant,a[0]/determinant];
        let metric = [quadratic(triangle.metric,x,x),quadratic(triangle.metric,x,y),quadratic(triangle.metric,y,y)];
        let scale = (0.5*(metric[0]+metric[2]+(metric[0]-metric[2]).hypot(2.*metric[1]))*(1.+1e-10)).sqrt();
        if !scale.is_finite() || !dominates([scale*scale,0.,scale*scale],metric) { return Err("Island transform reduces a source footprint".into()); }
        inputs.push(Some((uv,scale)));
        let keys: [Vec<u32>;3] = std::array::from_fn(|i| {
            let vertex = triangle.first+i;
            let mut key = vec![triangle.surface as u32];
            key.extend(surface.positions[vertex].map(f32::to_bits));
            key.extend(surface.uv[vertex].map(f32::to_bits));
            key.extend(surface.normals[vertex].map(f32::to_bits));
            key.extend(surface.tangents[vertex].map(f32::to_bits));
            key.extend(surface.colors[vertex].map(u32::from));
            key.extend(surface.auxiliary[vertex].iter().map(|&value| value as u32));
            key
        });
        for edge in 0..3 {
            let (a,b) = (keys[edge].clone(),keys[(edge+1)%3].clone());
            let key = if a < b { (a,b) } else { (b,a) };
            edges.entry(key).or_default().push(index);
        }
    }
    let pairs: Vec<_> = edges.values().flat_map(|members| members.iter().enumerate().flat_map(move |(i,&a)| members[i+1..].iter().map(move |&b| (a,b)))).collect();
    let mut parents: Vec<_> = (0..charts.len()).collect();
    let root = |mut index: usize, parents: &[usize]| { while parents[index] != index { index=parents[index]; } index };
    let mut groups: Vec<_> = (0..charts.len()).map(|index| vec![index]).collect();
    let mut costs: Vec<_> = charts.iter().map(|chart| (chart.width+2*GUTTER)*(chart.height+2*GUTTER)).collect();
    for &(a,b) in &pairs {
        let (mut a,mut b) = (root(a,&parents),root(b,&parents));
        if a == b { continue; }
        if groups[a].iter().any(|&left| groups[b].iter().any(|&right| overlaps(inputs[left].unwrap().0,inputs[right].unwrap().0))) { continue; }
        let members: Vec<_> = groups[a].iter().chain(&groups[b]).copied().collect();
        let scale = members.iter().map(|&index| inputs[index].unwrap().1).fold(0.,f64::max);
        let lower: [f64;2] = std::array::from_fn(|axis| members.iter().flat_map(|&index| inputs[index].unwrap().0).map(|uv| uv[axis]).fold(f64::INFINITY,f64::min));
        let upper: [f64;2] = std::array::from_fn(|axis| members.iter().flat_map(|&index| inputs[index].unwrap().0).map(|uv| uv[axis]).fold(f64::NEG_INFINITY,f64::max));
        let width = ((upper[0]-lower[0])*scale).ceil() as usize;
        let height = ((upper[1]-lower[1])*scale).ceil() as usize;
        if width+2*GUTTER > 8192 || height+2*GUTTER > 8192 { continue; }
        let cost = (width.max(2)+2*GUTTER)*(height.max(2)+2*GUTTER);
        if cost > costs[a]+costs[b] { continue; }
        if groups[a].len() < groups[b].len() { std::mem::swap(&mut a,&mut b); }
        parents[b]=a;
        let moved = std::mem::take(&mut groups[b]);
        groups[a].extend(moved);
        costs[a]=cost;
    }
    let original_count = charts.len();
    let mut charts: Vec<_> = charts.into_iter().map(Some).collect();
    let mut result = Vec::new();
    for members in groups.into_iter().filter(|members| !members.is_empty()) {
        if members.len() == 1 { result.push(charts[members[0]].take().unwrap()); continue; }
        let scale = members.iter().map(|&index| inputs[index].unwrap().1).fold(0.,f64::max);
        let lower: [f64;2] = std::array::from_fn(|axis| members.iter().flat_map(|&index| inputs[index].unwrap().0).map(|uv| uv[axis]).fold(f64::INFINITY,f64::min));
        let mut triangles = Vec::new();
        let mut extent = [0_f64;2];
        for index in members {
            let mut chart = charts[index].take().unwrap();
            let triangle = &mut chart.triangles[0];
            triangle.points = inputs[index].unwrap().0.map(|uv| std::array::from_fn(|axis| (uv[axis]-lower[axis])*scale));
            for point in triangle.points { for axis in 0..2 { extent[axis]=extent[axis].max(point[axis]); } }
            triangles.extend(chart.triangles);
        }
        result.push(Chart {triangles,width:(extent[0].ceil() as usize).max(2),height:(extent[1].ceil() as usize).max(2),x:0,y:0});
    }
    println!("Merged {original_count} triangle charts into {} continuous nonoverlapping UV islands",result.len());
    if result.iter().flat_map(|chart| &chart.triangles).map(|triangle| triangle.copies.len()).sum::<usize>() != source_triangles {
        return Err("Island assembly changed the source triangle count".into());
    }
    Ok(result)
}

fn quadratic(metric: [f64;3], left: [f64;2], right: [f64;2]) -> f64 {
    metric[0]*left[0]*right[0]+metric[1]*(left[0]*right[1]+left[1]*right[0])+metric[2]*left[1]*right[1]
}

fn dominates(metric: [f64;3], source: [f64;3]) -> bool {
    let [a,b,c] = std::array::from_fn(|i| metric[i]-source[i]);
    let minimum = 0.5*(a+c-(a-c).hypot(2.*b));
    let tolerance = metric.into_iter().chain(source).map(f64::abs).fold(1.,f64::max)*1e-10;
    minimum >= -tolerance
}

fn footprint(coordinates: &[[[f32;2];3]]) -> Result<[f64;3]> {
    let mut metrics = Vec::new();
    for uv in coordinates {
        let left: [f64;2] = std::array::from_fn(|axis| uv[1][axis] as f64-uv[0][axis] as f64);
        let right: [f64;2] = std::array::from_fn(|axis| uv[2][axis] as f64-uv[0][axis] as f64);
        let dot = |a: [f64;2],b: [f64;2]| a[0]*b[0]+a[1]*b[1];
        let metric = [dot(left,left),dot(left,right),dot(right,right)];
        if !metric.iter().all(|value| value.is_finite()) { return Err("Non-finite material UV footprint".into()); }
        if !metrics.contains(&metric) { metrics.push(metric); }
    }
    let mut best: [f64;3] = std::array::from_fn(|i| metrics.iter().map(|metric| metric[i]).sum());
    let determinant = |metric: [f64;3]| (metric[0]*metric[2]-metric[1]*metric[1]).max(0.);
    for &base in &metrics {
        let scale = base[0].max(base[2]);
        if base[0] <= 0. || determinant(base) <= scale*scale*1e-12 { continue; }
        let root = base[0].sqrt();
        let remainder = (base[2]-base[1]*base[1]/base[0]).sqrt();
        let (p,q,r) = (1./root,-base[1]/(base[0]*remainder),1./remainder);
        let mut factor = 1_f64;
        for &source in &metrics {
            let a = p*p*source[0];
            let b = p*(q*source[0]+r*source[1]);
            let c = q*q*source[0]+2.*q*r*source[1]+r*r*source[2];
            factor = factor.max(0.5*(a+c+(a-c).hypot(2.*b)));
        }
        let candidate = base.map(|value| value*factor*(1.+1e-10));
        if candidate.iter().all(|value| value.is_finite()) && determinant(candidate) < determinant(best)
            && metrics.iter().all(|&source| dominates(candidate,source)) { best=candidate; }
    }
    if !metrics.iter().all(|&source| dominates(best,source)) { return Err("Material UV metric reduces a source directional footprint".into()); }
    Ok(best)
}

struct Recipe {
    special: u8,
    special_parms: [[f32;4];3],
    inputs: Vec<Input>,
    base_specular: bool,
    geometric_normal: bool,
    second_additive: bool,
    second_normal: u8,
    second_reveal: bool,
    second_specular: bool,
    second_parms: [f32;4],
    third_multiply: bool,
    layer_cutout: bool,
    decal: bool,
    multiply: bool,
    additive: bool,
    blend_color: bool,
    reveal: bool,
    layer_specular: bool,
    base_alpha_response: bool,
    parms: [f32;4],
}

impl Recipe {
    fn color_operations(&self) -> bool { self.multiply || self.additive || self.blend_color || self.geometric_normal }
    fn color_only_layer(&self) -> bool { self.color_operations() && self.second_normal == 0 }

    fn auxiliary(&self, surface: &Surface, index: usize) -> Result<([f32;2],[f32;2],[f32;4],[f32;4],[f32;2])> {
        if self.third_multiply {
            let bytes = surface.auxiliary.get(index).ok_or("Third layered world UV is missing")?;
            if bytes.len() != 16 { return Err("Three-color layered vertex requires sixteen auxiliary bytes".into()); }
            let uv = |offset| [half::f16::from_bits(u16_at(bytes,offset)).to_f32(),half::f16::from_bits(u16_at(bytes,offset+2)).to_f32()];
            let (uv1,uv2,uv3) = (uv(0),uv(4),uv(8));
            if uv1.into_iter().chain(uv2).chain(uv3).any(|value| !value.is_finite()) { return Err("Third layered world UV is not finite".into()); }
            let frame = std::array::from_fn(|i| bytes[12+i] as f32/255.);
            return Ok((uv1,uv2,frame,frame,uv3));
        }
        if self.second_normal != 2 {
            let (uv1,uv2,frame) = auxiliary(surface,index,self.decal,self.color_only_layer())?;
            return Ok((uv1,uv2,frame,frame,[0.;2]));
        }
        let bytes = surface.auxiliary.get(index).ok_or("Two-normal layered world vertex has no auxiliary data")?;
        if bytes.len() != 16 { return Err("Two-normal layered vertex requires sixteen auxiliary bytes".into()); }
        let uv = |offset| [half::f16::from_bits(u16_at(bytes,offset)).to_f32(),half::f16::from_bits(u16_at(bytes,offset+2)).to_f32()];
        let (uv1,uv2) = (uv(0),uv(4));
        if uv1.into_iter().chain(uv2).any(|value| !value.is_finite()) { return Err("Two-normal layered UV is not finite".into()); }
        Ok((uv1,uv2,std::array::from_fn(|i| bytes[8+i] as f32/255.),std::array::from_fn(|i| bytes[12+i] as f32/255.),[0.;2]))
    }

    fn second_color_index(&self) -> usize { if self.color_operations() {3} else {4} }
    fn second_normal_index(&self) -> usize { self.second_color_index()+1 }
    fn layer_specular_index(&self) -> usize { 4+usize::from(self.decal)+usize::from(self.second_normal != 0) }
    fn second_specular_index(&self) -> usize {
        if self.color_operations() {5} else {self.layer_specular_index()+usize::from(self.layer_specular)}
    }
    fn has_specular(&self) -> bool { self.base_specular || self.layer_specular || self.second_specular }

    fn fritz_inputs(&self, uv: [f32;2]) -> [[f32;4];3] {
        let base=self.inputs[0].sample(uv);let mut color=base;
        let mut specular=self.inputs[2].sample(uv);let mut glow=self.inputs[3].sample(uv);
        let state=self.special_parms[1][0];
        if state>0. {
            let warp=self.inputs[4].sample(uv.map(|value| value*self.second_parms[3]));
            let offset:[f32;2]=std::array::from_fn(|axis| (warp[axis]*2.-1.)*self.second_parms[2]);
            let warped_uv=std::array::from_fn(|axis| uv[axis]+offset[axis]);
            let warped=self.inputs[0].sample(warped_uv);
            let luminance=warped[0]*0.3+warped[1]*0.59+warped[2]*0.11;
            let luma=luminance*luminance*self.special_parms[0][0];
            color=std::array::from_fn(|axis| if axis == 3 {warped[3]} else {base[axis]+state*(luma-base[axis])});
            glow=self.inputs[3].sample(warped_uv);
            let sparkle_uv=std::array::from_fn(|axis| uv[axis]*self.parms[3]
                +(self.special_parms[2][3]*self.second_parms[axis]*self.parms[3]).rem_euclid(1.)+offset[axis]);
            let sparkle=self.inputs[5].sample(sparkle_uv);
            let vertical=uv[1]*uv[1]*uv[1]*sparkle[3];
            for axis in 0..3 {let target=luma+vertical*(sparkle[axis]-luma);glow[axis]=base[axis]+state*(target-base[axis]);}
            let warped=self.inputs[2].sample(warped_uv);let luminance=warped[0]*0.3+warped[1]*0.59+warped[2]*0.11;
            for axis in 0..3 {specular[axis]+=state*(luminance-specular[axis]);}
        }
        [color,specular,glow]
    }

    fn emission_pixel(&self, uv: [f32;2], weight: f32) -> [f32;4] {
        if self.special == 2 {
            return std::array::from_fn(|channel| if channel == 3 {1.} else {(self.second_parms[channel]*self.second_parms[3]).powi(2)*weight});
        }
        let mut source=if self.special == 16 {self.fritz_inputs(uv)[2]} else {self.inputs[if self.special == 8 {2} else {3}].sample(uv)};
        if self.special == 64 {for channel in 0..4 {source[channel]*=self.special_parms[1][channel];}}
        std::array::from_fn(|channel| if channel == 3 {1.} else {(source[channel]*source[3]).powi(2)*weight})
    }

    #[cfg(test)]
    fn sample_special(&self, uv: [f32;2]) -> Result<[[f32;4];4]> {
        let sample = |index:usize,uv| self.inputs[index].sample(uv);
        let decode = |value:f32| value*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204);
        let normal = |slope:[f32;2]| {
            let length=(slope[0]*slope[0]+slope[1]*slope[1]+1.).sqrt();
            [slope[0]/length,slope[1]/length,1./length,1.]
        };
        let fit = |color:[f32;3],normal,rgb:[f32;3],gloss,emission| {
            let (color,metallic,roughness)=crate::material::pbr_pixel(color,rgb,gloss);
            [[color[0],color[1],color[2],1.],normal,[1.,roughness,metallic,emission],[rgb[0],rgb[1],rgb[2],gloss]]
        };
        if self.special == 16 {
            let [color,response,_]=self.fritz_inputs(uv);let n=sample(1,uv);
            let intensity=self.parms[2]*self.special_parms[1][1];
            let mut pixel=fit(std::array::from_fn(|axis| color[axis]*color[axis]),normal([decode(n[0]),decode(n[1])]),
                std::array::from_fn(|axis| response[axis]*response[axis]),response[3],intensity*intensity);
            if self.special_parms[0][1] != 0. {pixel[0][3]=color[3];}
            return Ok(pixel);
        }
        if matches!(self.special,4 | 8 | 32 | 64) {
            let base=sample(0,uv);let n=sample(1,uv);
            let color=std::array::from_fn(|axis| base[axis]*base[axis]);let normal=normal([decode(n[0]),decode(n[1])]);
            if self.special == 8 {
                return Ok(fit(color,normal,[0.04;3],base[3],self.parms[0]*self.parms[0]));
            }
            if self.special == 32 {
                let response=sample(2,uv);let intensity=if self.special_parms[1][2]>=self.second_parms[2] {self.parms[3]} else {0.};
                let mut pixel=fit(color,normal,std::array::from_fn(|axis| response[axis]*response[axis]),response[3],intensity*intensity);
                if self.second_parms[3] != 0. {pixel[0][3]=base[3];}
                return Ok(pixel);
            }
            let response=sample(2,uv);let rgb=std::array::from_fn(|axis| (response[axis]*self.parms[0]).powi(2));
            let emission=if self.special == 64 {self.parms[3]} else {self.parms[2]*self.parms[2]*self.parms[3]};
            let mut pixel=fit(color,normal,rgb,response[3]*self.parms[1],emission*emission);
            if self.special == 4 && self.special_parms[0][0] != 0. {pixel[0][3]=base[3];}
            return Ok(pixel);
        }
        if self.special == 1 {
            let first_uv=std::array::from_fn(|axis| uv[axis]*self.parms[axis]);
            let second_uv=std::array::from_fn(|axis| uv[axis]*self.parms[axis+2]);
            let mask=sample(0,uv);let weight=mask[1]*mask[1];
            let base=sample(1,uv);let first=sample(2,first_uv);let second=sample(3,second_uv);
            let slope=std::array::from_fn(|axis| decode(base[axis])*self.second_parms[1]
                +decode(first[axis])*((1.-weight)*self.second_parms[0])+decode(second[axis])*(weight*self.second_parms[3]));
            let first=sample(4,first_uv);let second=sample(5,second_uv);
            let color=std::array::from_fn(|axis| (first[axis]+weight*(second[axis]-first[axis])).powi(2));
            let normal=normal(slope);
            if self.base_specular {
                let first=sample(6,first_uv);let second=sample(7,second_uv);
                let response:[f32;4]=std::array::from_fn(|axis| first[axis]+weight*(second[axis]-first[axis]));
                return Ok(fit(color,normal,std::array::from_fn(|axis| response[axis+1].powi(2)),response[0],1.));
            }
            return Ok([[color[0],color[1],color[2],1.],normal,[1.,1.,0.,1.],[0.;4]]);
        }
        if uv.iter().any(|value| *value<0.) {return Err("Water reveal source logarithm has a negative coordinate".into());}
        let time=self.special_parms[2][3];
        let motion=(time*f32::from_bits(0xbe116d3f)).rem_euclid(1.);
        let warp=sample(2,[uv[0]*f32::from_bits(0x4035c88e)*f32::from_bits(0x3f7f41f2),(uv[1]*f32::from_bits(0x4035c88e)+motion)*f32::from_bits(0xbdb851ec)]);
        let power=warp[0]*0.1+1.;
        let water_uv:[f32;2]=std::array::from_fn(|axis| (uv[axis].powf(power)-0.5)/self.parms[axis+2]+0.5);
        let scroll=time*self.parms[0];
        let first=sample(3,[water_uv[0]-scroll*f32::from_bits(0xbc11fcab),water_uv[1]-scroll*f32::from_bits(0x3f4cc98c)]);
        let second=sample(3,[water_uv[0]-(scroll*f32::from_bits(0xbc6d3a96)+0.5),water_uv[1]-(scroll*f32::from_bits(0x3fa663c2)+0.5)]);
        let sum:[f32;2]=std::array::from_fn(|axis| decode(first[axis])+decode(second[axis]));
        let length=(sum[0]*sum[0]+sum[1]*sum[1]+4.).sqrt();
        let z=2./length;let water=[sum[0]/length/z,sum[1]/length/z,1.];
        let positive=(water[0]+water[1])*0.5>0.;
        let response=sample(5,uv);let temporal=sample(0,uv)[0];let state=self.special_parms[1][2];
        let mut reveal=[((temporal-(state+f32::from_bits(0x3d75c28f)))*f32::from_bits(0xc1055555)).clamp(0.,1.),
            ((temporal-(state+f32::from_bits(0xbcf5c28f)))*f32::from_bits(0x41855555)).clamp(0.,1.)];
        reveal=reveal.map(|value| value*value*(3.-2.*value));
        let bias=f32::from_bits(if positive {0x3e5b6dca} else {0x3e92491b});
        let rgb=std::array::from_fn(|axis| {let wet=response[axis]+bias;(wet+reveal[0]*(response[axis]-wet)).powi(2)});
        let wet_gloss=if positive {response[3]} else {1.};let gloss=wet_gloss+reveal[0]*(response[3]-wet_gloss);
        let base=sample(1,uv);let slope=std::array::from_fn(|axis| {let base=decode(base[axis]);let wet=base*water[2]+water[axis]*self.parms[1];wet+reveal[0]*(base-wet)});
        let warped=sample(4,std::array::from_fn(|axis| (uv[axis]-water[axis]*water[2]).clamp(0.,1.)));
        let base=sample(4,uv)[0];let tint=[0x3f3bc148,0x3f49c91d,0x3f649247].map(f32::from_bits);
        let color=std::array::from_fn(|axis| {let wet=warped[axis]*tint[axis];(wet+reveal[0]*(self.special_parms[0][axis]*base-wet)).powi(2)});
        Ok(fit(color,normal(slope),rgb,gloss,(reveal[0]*reveal[1]).powi(2)))
    }

    fn input_uv(&self, index: usize) -> usize {
        if self.third_multiply && index + 1 == self.inputs.len() {3}
        else if index == 0 || !self.geometric_normal && index == 1 || self.base_specular && index + 1 == self.inputs.len() { 0 }
        else if self.second_normal != 0 && index == self.second_normal_index()
            || self.second_specular && index == self.second_specular_index()
            || self.decal && index == self.second_color_index() { 2 }
        else { 1 }
    }

    #[cfg(test)]
    fn reveal_weight(alpha: f32, vertex: f32, parms: [f32;4]) -> Result<f32> {
        let a = vertex*f32::from_bits(0x3f7f7cee)+f32::from_bits(0x3a83126f);
        let q = 1.-a;
        let low = (q-parms[0]*(a.log2()*parms[1]).exp2()).clamp(0.,1.);
        let high = (q+parms[0]*(q.log2()*parms[1]).exp2()).clamp(0.,1.);
        if high <= low { return Err("Layered reveal thresholds have no interval".into()); }
        Ok(((alpha-low)/(high-low)).clamp(0.,1.))
    }

    #[cfg(test)]
    fn sample(&self, uv0: [f32;2], uv1: [f32;2], uv2: [f32;2], green: f32, blue: f32, transform: [f32;4]) -> Result<[[f32;4];4]> {
        self.sample_frames(uv0,uv1,uv2,green,blue,transform,transform,[0.;4])
    }

    #[cfg(test)]
    fn sample_frames(&self, uv0: [f32;2], uv1: [f32;2], uv2: [f32;2], green: f32, blue: f32, transform: [f32;4], second_transform: [f32;4], third: [f32;4]) -> Result<[[f32;4];4]> {
        if self.special != 0 { return self.sample_special(uv0); }
        let inputs = &self.inputs;
        if self.color_operations() {
            let base = inputs[0].sample(uv0);
            let layer = inputs[if self.geometric_normal {1} else {2}].sample(uv1);
            let second = if self.decal { inputs[self.second_color_index()].sample(uv2) } else { [1.;4] };
            let second_weight = if self.second_reveal {Self::reveal_weight(second[3],blue,self.second_parms)?} else {blue*second[3]};
            let tint: [f32;3] = std::array::from_fn(|c| 1.+green*(layer[c]-1.));
            let mut color = std::array::from_fn(|c| if c == 3 { 1. } else {
                if self.blend_color {
                    let first = base[c]+green*layer[3]*(layer[c]-base[c]);
                    let weight = if self.decal {second_weight} else {0.};
                    (first+weight*(second[c]-first)).powi(2)
                }
                else if self.additive { ((base[c]+layer[c]*green*layer[3])*(1.+blue*(second[c]-1.))).powi(2) }
                else { (base[c]*tint[c]*(1.+blue*(second[c]-1.))).powi(2) }
            });
            let mut response = if self.base_alpha_response {
                let second_tint: [f32;3] = std::array::from_fn(|c| 1.+blue*(second[c]-1.));
                let gloss = if self.additive && self.decal {base[3]*(second_tint[0]*0.3+second_tint[1]*0.6+second_tint[2]*0.1)}
                    else if self.additive || self.blend_color {base[3]} else {base[3]*(tint[0]*0.3+tint[1]*0.6+tint[2]*0.1)};
                [1.,(2./(2_f32.powf(13.*gloss)+2.)).powf(0.25),0.,gloss]
            } else { [1.,1.,0.,1.] };
            let mut specular = [0.;4];
            if self.has_specular() {
                let k = f32::from_bits(0x3e4ccccd);
                let mut source = if self.base_specular {inputs.last().unwrap().sample(uv0)} else {[k,k,k,base[3]]};
                if self.second_specular {
                    let second = inputs[self.second_specular_index()].sample(uv2);
                    source=std::array::from_fn(|c| source[c]+second_weight*(second[c]-source[c]));
                }
                let rgb = std::array::from_fn(|c| source[c]*source[c]);
                let (base,metallic,roughness) = crate::material::pbr_pixel([color[0],color[1],color[2]],rgb,source[3]);
                color=[base[0],base[1],base[2],1.];
                response=[1.,roughness,metallic,1.];
                specular=[rgb[0],rgb[1],rgb[2],source[3]];
            }
            if self.geometric_normal { return Ok([color,[0.,0.,1.,1.],response,specular]); }
            let normal = inputs[1].sample(uv0);
            let decode = |value: f32| value*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204);
            let mut slope = [normal[0],normal[1]].map(decode);
            if self.second_normal == 1 {
                let second_normal = inputs[self.second_normal_index()].sample(uv2);
                let s = [decode(second_normal[0]),decode(second_normal[1])];
                let rotated = [s[0]*transform[0]+s[1]*transform[1],s[0]*transform[2]+s[1]*transform[3]];
                let weight = second_weight;
                slope=std::array::from_fn(|axis| slope[axis]+weight*(rotated[axis]-slope[axis]));
            }
            let length = (slope[0]*slope[0]+slope[1]*slope[1]+1.).sqrt();
            return Ok([color,[slope[0]/length,slope[1]/length,1./length,1.],response,specular]);
        }
        let (c0,c1,n0,n1) = (inputs[0].sample(uv0),inputs[2].sample(uv1),inputs[1].sample(uv0),inputs[3].sample(uv1));
        let weight = if self.reveal {Self::reveal_weight(c1[3],green,self.parms)?}
            else if self.layer_cutout {if c1[3]*green >= 0.5 {1.} else {0.}} else {c1[3]*green};
        let decal = if self.decal { inputs[self.second_color_index()].sample(uv2) } else { [1.;4] };
        let second_weight = if self.second_reveal {Self::reveal_weight(decal[3],blue,self.second_parms)?} else {blue*decal[3]};
        let third_color = if self.third_multiply {inputs.last().unwrap().sample([third[0],third[1]])} else {[1.;4]};
        let third_tint: [f32;3] = std::array::from_fn(|c| 1.+third[2]*(third_color[c]-1.));
        let mut color = [1.;4];
        if self.layer_cutout && weight == 1. {color[3]=c1[3];}
        for c in 0..3 {
            let first = c0[c]+weight*(c1[c]-c0[c]);
            let encoded = if self.second_additive {first+blue*decal[3]*decal[c]}
                else if self.second_normal != 0 {first+second_weight*(decal[c]-first)}
                else {first*(1.+blue*(decal[c]-1.))};
            color[c] = (encoded*third_tint[c]).powi(2);
        }
        let decode = |value: f32| value*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204);
        let s0 = [decode(n0[0]),decode(n0[1])];
        let s1 = [decode(n1[0]),decode(n1[1])];
        let rotated = [s1[0]*transform[0]+s1[1]*transform[1],s1[0]*transform[2]+s1[1]*transform[3]];
        let mut slope = [s0[0]+weight*(rotated[0]-s0[0]),s0[1]+weight*(rotated[1]-s0[1])];
        if self.second_normal == 2 {
            let second = inputs[self.second_normal_index()].sample(uv2);
            let s = [decode(second[0]),decode(second[1])];
            let rotated = [s[0]*second_transform[0]+s[1]*second_transform[1],s[0]*second_transform[2]+s[1]*second_transform[3]];
            let weight = second_weight;
            slope=std::array::from_fn(|axis| slope[axis]+weight*(rotated[axis]-slope[axis]));
        }
        let length = (slope[0]*slope[0]+slope[1]*slope[1]+1.).sqrt();
        let normal = [slope[0]/length,slope[1]/length,1./length,1.];
        let (mut response,mut specular) = ([1.,1.,0.,1.],[0.;4]);
        if self.has_specular() {
            let k = f32::from_bits(0x3e4ccccd);
            let base_gloss = if self.base_alpha_response { c0[3] } else { 0. };
            let base = if self.base_specular {inputs.last().unwrap().sample(uv0)} else {[k,k,k,base_gloss]};
            let mut mixed = if self.layer_specular {
                let layer = inputs[self.layer_specular_index()].sample(uv1);
                std::array::from_fn(|c| base[c]+weight*(layer[c]-base[c]))
            } else {base};
            if self.second_specular {
                let second = inputs[self.second_specular_index()].sample(uv2);
                mixed=std::array::from_fn(|c| mixed[c]+second_weight*(second[c]-mixed[c]));
            }
            let rgb = std::array::from_fn(|c| mixed[c]*mixed[c]);
            let tint: [f32;3] = std::array::from_fn(|c| 1.+blue*(decal[c]-1.));
            let gloss = mixed[3]*if self.third_multiply {third_tint[0]*0.3+third_tint[1]*0.6+third_tint[2]*0.1}
                else if self.decal && !self.second_additive && self.second_normal == 0 {tint[0]*0.3+tint[1]*0.6+tint[2]*0.1} else {1.};
            let (base,metallic,roughness) = crate::material::pbr_pixel([color[0],color[1],color[2]],rgb,gloss);
            color = [base[0],base[1],base[2],1.];
            response = [1.,roughness,metallic,1.];
            specular = [rgb[0],rgb[1],rgb[2],gloss];
        } else if self.base_alpha_response {
            response = [1.,(2./(2_f32.powf(13.*c0[3])+2.)).powf(0.25),0.,c0[3]];
        }
        Ok([color,normal,response,specular])
    }
}

fn auxiliary(surface: &Surface, index: usize, decal: bool, color_only_layer: bool) -> Result<([f32; 2], [f32; 2], [f32; 4])> {
    let bytes = surface.auxiliary.get(index).ok_or("Layered world vertex has no auxiliary data")?;
    let size = if color_only_layer { if decal {8} else {4} } else if decal {12} else {8};
    if bytes.len() != size { return Err(format!("Layered world vertex requires {size} auxiliary bytes, found {}", bytes.len()).into()); }
    let uv = [half::f16::from_bits(u16_at(bytes, 0)).to_f32(), half::f16::from_bits(u16_at(bytes, 2)).to_f32()];
    if !uv.iter().all(|v| v.is_finite()) { return Err("Layered world UV is not finite".into()); }
    let uv2 = if decal { [half::f16::from_bits(u16_at(bytes, 4)).to_f32(), half::f16::from_bits(u16_at(bytes, 6)).to_f32()] } else { uv };
    if !uv2.iter().all(|v| v.is_finite()) { return Err("Layered decal UV is not finite".into()); }
    Ok((uv, uv2, if color_only_layer { [1.,0.,0.,1.] } else { std::array::from_fn(|i| bytes[size - 4 + i] as f32 / 255.) }))
}

fn constant_frame(surface: &Surface, first: usize) -> bool {
    (1..3).all(|i| surface.normals[first+i] == surface.normals[first] && surface.tangents[first+i] == surface.tangents[first])
}

fn dot(a: [f64;3], b: [f64;3]) -> f64 { a.into_iter().zip(b).map(|(a,b)| a*b).sum() }

fn cross(a: [f64;3], b: [f64;3]) -> [f64;3] {
    [a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]
}

fn unit(value: [f64;3]) -> Result<[f64;3]> {
    let length = dot(value,value).sqrt();
    if !length.is_finite() || length <= 1e-12 { return Err("Degenerate layered normal frame".into()); }
    Ok(value.map(|v| v/length))
}

struct NormalFrame {
    normals: [[f64;3];3],
    tangents: [[f64;3];3],
    bitangents: [[f64;3];3],
    sign: f64,
}

impl NormalFrame {
    fn read(surface: &Surface, first: usize) -> Result<Self> {
        let normals = std::array::from_fn(|i| surface.normals[first+i].map(f64::from));
        let tangents = std::array::from_fn(|i| std::array::from_fn(|axis| surface.tangents[first+i][axis] as f64));
        let sign = surface.tangents[first][3] as f64;
        if (0..3).any(|i| !matches!(surface.tangents[first+i][3], -1. | 1.)) {
            return Err(format!("Layered surface {} triangle {} has invalid tangent handedness",surface.name,first/3).into());
        }
        let bitangents = std::array::from_fn(|i| cross(normals[i],tangents[i]).map(|v| v*surface.tangents[first+i][3] as f64));
        Ok(Self {normals,tangents,bitangents,sign})
    }

    fn convert(&self, weights: [f64;3], normal: [f32;4]) -> Result<([f32;4],[f64;2])> {
        let interpolate = |values: [[f64;3];3]| std::array::from_fn(|axis| (0..3).map(|i| values[i][axis]*weights[i]).sum::<f64>());
        let n = interpolate(self.normals);
        let t = interpolate(self.tangents);
        let b = interpolate(self.bitangents);
        let source = unit(std::array::from_fn(|axis| t[axis]*normal[0] as f64+b[axis]*normal[1] as f64+n[axis]*normal[2] as f64))?;
        let destination_n = unit(n)?;
        let destination_t = unit(t)?;
        let destination_b = cross(destination_n,destination_t).map(|v| v*self.sign);
        let inverse_rows = [cross(destination_b,destination_n),cross(destination_n,destination_t),cross(destination_t,destination_b)];
        let determinant = dot(destination_t,inverse_rows[0]);
        if !determinant.is_finite() || determinant.abs() <= 1e-12 { return Err("Singular layered destination normal frame".into()); }
        let converted = unit(inverse_rows.map(|row| dot(source,row)/determinant))?;
        if converted[2] <= 0. { return Err("Layered destination normal exceeds the two-channel positive-Z hemisphere".into()); }
        let converted = converted.map(|v| v as f32);
        let raster_b = cross(n,t).map(|v| v*self.sign);
        let direction = |t: [f64;3],b: [f64;3],n: [f64;3]| unit(std::array::from_fn(|axis| t[axis]*converted[0] as f64+b[axis]*converted[1] as f64+n[axis]*converted[2] as f64));
        let angle = |value: [f64;3]| dot(cross(source,value),cross(source,value)).sqrt().atan2(dot(source,value)).to_degrees();
        let solari = direction(destination_t,destination_b,destination_n)?;
        let raster = direction(t,raster_b,n)?;
        Ok(([converted[0],converted[1],converted[2],1.],[angle(solari),angle(raster)]))
    }
}

fn periodic_key(surface: &Surface, first: usize, color_only_layer: bool) -> Result<Option<Vec<u32>>> {
    if !constant_frame(surface,first) { return Ok(None); }
    let auxiliary = (0..3).map(|i| auxiliary(surface,first+i,false,color_only_layer)).collect::<Result<Vec<_>>>()?;
    if !(1..3).all(|i| surface.colors[first+i][1] == surface.colors[first][1] && auxiliary[i].2 == auxiliary[0].2) { return Ok(None); }
    let offsets: [[f32;2];3] = std::array::from_fn(|i| std::array::from_fn(|axis| auxiliary[i].0[axis]-surface.uv[first+i][axis]));
    if offsets[1] != offsets[0] || offsets[2] != offsets[0] { return Ok(None); }
    let mut key = vec![surface.colors[first][1] as u32];
    key.extend(auxiliary[0].2.map(f32::to_bits));
    key.extend(offsets[0].map(|value| value.rem_euclid(1.).to_bits()));
    Ok(Some(key))
}

fn periodic(native: &Value, recipe: &Recipe, baker: &gpu::Baker, key: &[u32], uri: &str, pack: &mut Pack) -> Result<Value> {
    let width = recipe.inputs.iter().map(|image| image.width).max().ok_or("Periodic material has no inputs")?;
    let height = recipe.inputs.iter().map(|image| image.height).max().unwrap();
    let mut outputs: [Vec<[f32;4]>;4] = std::array::from_fn(|_| Vec::with_capacity(width*height));
    let green = key[0] as f32/255.;
    let transform: [f32;4] = std::array::from_fn(|axis| 2.*f32::from_bits(key[axis+1])-1.);
    let offset: [f32;2] = std::array::from_fn(|axis| f32::from_bits(key[axis+5]));
    for start in (0..width*height).step_by(262_144) {
        let mut samples = vec![[0.;20]; (width*height-start).min(262_144)];
        parallel_chunks_mut(&mut samples, 4096, |base, samples| {
            for (index, sample) in samples.iter_mut().enumerate() {
                let index = start+base+index;
                let uv0 = [((index%width) as f32+0.5)/width as f32,((index/width) as f32+0.5)/height as f32];
                let uv1: [f32;2] = std::array::from_fn(|axis| uv0[axis]+offset[axis]);
                *sample = [uv0[0],uv0[1],uv1[0],uv1[1],uv0[0],uv0[1],green,0.,transform[0],transform[1],transform[2],transform[3],transform[0],transform[1],transform[2],transform[3],0.,0.,0.,0.];
            }
            Ok(())
        })?;
        for pixel in baker.sample(&samples)? {
            for (output,pixel) in outputs.iter_mut().zip(pixel) { output.push(pixel); }
        }
    }
    let material = store(native,uri,width,height,outputs,20,0,pack)?;
    println!("Baked shared periodic recipe {uri}: {width}x{height}, source green {}",key[0]);
    Ok(material)
}

fn mip_chain(width: usize, height: usize, pixels: Vec<[f32; 4]>, role: &str) -> Result<texture::Image> {
    if width == 0 || height == 0 || width.checked_mul(height) != Some(pixels.len()) {
        return Err("Baked material dimensions do not match pixel count".into());
    }
    if pixels.iter().flatten().any(|value| !value.is_finite() || role != "normal" && *value < 0.) {
        return Err("Baked material has a non-finite or negative channel".into());
    }
    let hdr = matches!(role,"color" | "native-specular") && pixels.iter().any(|pixel| pixel.iter().any(|&value| value > 1.));
    let mut image = texture::Image { width, height, depth:1, faces:1, format:if hdr {19} else {1}, flags:0, levels:Vec::new() };
    let (mut w, mut h, mut pixels) = (width, height, pixels);
    loop {
        let stride = if hdr {8} else {4};
        let mut bytes = vec![0;w*h*stride];
        parallel_chunks_mut(&mut bytes, 4096*stride, |offset, bytes| {
        for (bytes,pixel) in bytes.chunks_exact_mut(stride).zip(&pixels[offset/stride..]) {
            let mut value = *pixel;
            if role == "normal" {
                let length = value[..3].iter().map(|v| v*v).sum::<f32>().sqrt();
                if length < 1e-8 { value = [0., 0., 1., 1.]; }
                else { for channel in &mut value[..3] { *channel /= length; } }
                for channel in &mut value[..3] { *channel = *channel * 0.5 + 0.5; }
            } else if role == "color" && !hdr {
                for channel in &mut value[..3] {
                    *channel = if *channel <= 0.0031308 { *channel * 12.92 } else { 1.055 * channel.powf(1./2.4) - 0.055 };
                }
            }
            if hdr {
                if value.iter().any(|v| !v.is_finite() || !(0. ..=65504.).contains(v)) || role == "color" && value[3] > 1. {
                    return Err("Baked material pixel exceeds half-float color range".into());
                }
                for (bytes,component) in bytes.chunks_exact_mut(2).zip(value) { bytes.copy_from_slice(&half::f16::from_f32(component).to_le_bytes()); }
            } else {
                if value.iter().any(|v| !v.is_finite() || !(0. ..=1.).contains(v)) { return Err("Baked material pixel exceeds UNORM range".into()); }
                bytes.copy_from_slice(&value.map(|v| (v*255.).round() as u8));
            }
        }
        Ok(())
        })?;
        image.levels.push(bytes);
        if w == 1 && h == 1 { break; }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![[0.;4];nw*nh];
        parallel_chunks_mut(&mut next, 4096, |offset, next| {
        for (index,pixel) in next.iter_mut().enumerate() {
            let (x,y) = ((offset+index)%nw,(offset+index)/nw);
            let values = [pixels[(y*2).min(h-1)*w + (x*2).min(w-1)], pixels[(y*2).min(h-1)*w + (x*2+1).min(w-1)],
                pixels[(y*2+1).min(h-1)*w + (x*2).min(w-1)], pixels[(y*2+1).min(h-1)*w + (x*2+1).min(w-1)]];
            *pixel=std::array::from_fn(|c| values.iter().map(|v| v[c]).sum::<f32>() * 0.25);
        }
        Ok(())
        })?;
        (w, h, pixels) = (nw, nh, next);
    }
    Ok(image)
}

pub fn validate_native(native: &Value) -> Result<()> { recipe(native,None).map(|_| ()) }

fn special_family(family: &str) -> bool {
    emissive_family(family) || matches!(family,"mc_sw4_3d_cod7_tile_blend_spec_8w40z341" | "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6"
        | "mc_sw4_3d_cod7_tile_blend_q2q2zej6" | "mlv_sw4_3d_cod7_tile_blend_26zq1w22"
        | "mc_sw4_3d_water_on_surface_reveal_77320z68")
}

fn emissive_family(family: &str) -> bool {
    matches!(family,"mc_sw4_3d_phong_emissive_q0751958" | "mc_sw4_3d_phong_emissive_alcatraz_90892wq5"
        | "mc_sw4_3d_phong_emissive_alcatraz_8q551153" | "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e"
        | "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3" | "mc_sw4_3d_phong_emissive_fritz_76q5wqjw"
        | "mc_sw4_3d_phong_emissive_fritz_61728967" | "mc_sw4_3d_phong_emissive_script_multi_72q2fj2u"
        | "mc_sw4_3d_phong_emissive_script_multi_j05zz52w" | "mc_sw4_3d_emissive_tomb_qww536q2")
}

fn special_recipe(native: &Value, images: Option<&BTreeMap<String, texture::Image>>) -> Result<Recipe> {
    let family = native["technique_set"].as_str().ok_or("Surface family is missing")?;
    if emissive_family(family) {return emissive_recipe(native,images);}
    let water = family == "mc_sw4_3d_water_on_surface_reveal_77320z68";
    let specular = water || family.contains("tile_blend_spec");
    let programs = match family {
        "mc_sw4_3d_cod7_tile_blend_q2q2zej6" => ["sw4_3d_cod7_tile_blend_3e199cfe","sw4_3d_cod7_tile_blend_e88a8c75","sw4_3d_cod7_tile_blend_ff4d7872","sw4_3d_cod7_tile_blend_2d911f96"],
        "mlv_sw4_3d_cod7_tile_blend_26zq1w22" => ["sw4_3d_cod7_tile_blend_e11f113c","sw4_3d_cod7_tile_blend_378c01b7","sw4_3d_cod7_tile_blend_204bf5b0","sw4_3d_cod7_tile_blend_f2979254"],
        "mc_sw4_3d_cod7_tile_blend_spec_8w40z341" => ["sw4_3d_cod7_tile_blend_spec_6bd58c9b","sw4_3d_cod7_tile_blend_spec_d5cd3eb7","sw4_3d_cod7_tile_blend_spec_ff8a6704","sw4_3d_cod7_tile_blend_spec_b3b2a268"],
        "mlv_sw4_3d_cod7_tile_blend_spec_w8wezqe6" => ["sw4_3d_cod7_tile_blend_spec_e51efca4","sw4_3d_cod7_tile_blend_spec_5b064e88","sw4_3d_cod7_tile_blend_spec_7141173b","sw4_3d_cod7_tile_blend_spec_3d79d257"],
        "mc_sw4_3d_water_on_surface_reveal_77320z68" => ["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_water_on_surface_reveal_f3ca08bb","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_water_on_surface_reveal_6851d36b"],
        _ => return Err("Unreviewed special surface family".into()),
    };
    let bindings = if water {vec![(2782436735_u32,256),(1507003663,513),(3827006327,770),(1837877086,1027),(2695565377,1284),(2351529600,1541)]}
        else if specular {vec![(3976875907,256),(329745474,1025),(3400726183,515),(3069501348,770),(2643103145,1542),(3580385162,2052),(2415611304,1287),(830013099,1797)]}
        else {vec![(3976875907,256),(329745474,1025),(3400726183,515),(3069501348,770),(2643103145,1285),(3580385162,1540)]};
    let constants = if water {vec![(111253676_u32,124,4),(348546551,96,12),(1116993630,128,4),(1288488438,112,12),(3070667917,132,8),(3071481490,80,4),(3228582790,84,4)]}
        else {let mut values=vec![(523422290,92,4),(2569221807,96,4),(4005921460,88,4),(4266698135,100,4)];if specular {values.push((211931156,112,4));}values};
    let expected_routes = [(0,0),(1,2),(2,5),(3,1),(4,7),(5,6)];
    let route_count = if family.starts_with("mlv_") {6} else {5};
    for (offset,index) in [4_usize,6].into_iter().enumerate() {
        let technique = native["techniques"].as_array().ok_or("Special surface techniques are missing")?.iter().find(|value| value["index"] == index).ok_or("Special surface selected technique is missing")?;
        let passes = technique["passes"].as_array().ok_or("Special surface passes are missing")?;
        let state = native["state_entries"][index].as_u64().ok_or("Special surface state is missing")? as usize;
        if passes.len() != 1 || passes[0]["vertex_shader"] != format!("pimp_shader_{}.hlsl",programs[offset*2])
            || passes[0]["pixel_shader"] != format!("pimp_shader_{}.hlsl",programs[offset*2+1])
            || native["state_bits"][state] != json!([403867666_u32,13]) { return Err("Special surface selected programs or states differ from recovered source".into()); }
        let routes = passes[0]["vertex_routes"].as_array().ok_or("Special surface routes are missing")?;
        let args = passes[0]["arguments"].as_array().ok_or("Special surface arguments are missing")?;
        if routes.len() != route_count || routes.iter().zip(expected_routes).any(|(route,(source,destination))| route["source"] != source || route["destination"] != destination)
            || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
            || bindings.iter().any(|&(hash,slot)| !args.iter().any(|arg| arg["type"] == 2 && arg["value"] == hash && arg["location"] == slot && arg["buffer"] == 0 && arg["size"] == 1))
            || args.iter().filter(|arg| arg["type"] == 6).count() != constants.len()
            || constants.iter().any(|&(hash,location,size)| !args.iter().any(|arg| arg["type"] == 6 && arg["value"] == hash && arg["buffer"] == 2 && arg["location"] == location && arg["size"] == size)) {
            return Err("Special surface active bindings or vertex routes differ from recovered source".into());
        }
        if water {
            for (code,buffer,location) in [(172,1,416),(25,2,64)] {
                if !args.iter().any(|arg| arg["type"] == 5 && arg["value"] == 16777216_u32+code && arg["buffer"] == buffer && arg["location"] == location && arg["size"] == 16) {
                    return Err("Water reveal script/time binding differs from recovered source".into());
                }
            }
        } else if args.iter().filter(|arg| arg["type"] == 0).count() != 2 || [(404288371_u32,80),(2106384112,104)].into_iter().any(|(hash,location)| !args.iter().any(|arg| arg["type"] == 0 && arg["value"] == hash && arg["buffer"] == 2 && arg["location"] == location && arg["size"] == 8)) {
            return Err("Tile surface micro-coordinate scale binding differs from recovered source".into());
        }
    }
    let constant = |hash| -> Result<[f32;4]> {
        let value = native["constants"].as_array().ok_or("Special surface constants are missing")?.iter().find(|value| value["hash"] == hash).ok_or("Special surface authored constant is missing")?;
        let literal: [f32;4] = serde_json::from_value(value["literal"].clone())?;
        if literal.iter().any(|value| !value.is_finite()) {return Err("Special surface constant is not finite".into());}
        Ok(literal)
    };
    for &(hash,_,_) in &constants {constant(hash)?;}
    let (parms,second_parms,mut special_parms) = if water {
        let scale = constant(3070667917)?;
        if scale[0] == 0. || scale[1] == 0. {return Err("Water reveal scale has a zero divisor".into());}
        let edge=constant(348546551)?;let alteration=constant(1288488438)?;
        ([constant(3071481490)?[0],constant(1116993630)?[0],scale[0],scale[1]],
            [edge[0],edge[1],edge[2],constant(3228582790)?[0]],[[alteration[0],alteration[1],alteration[2],constant(111253676)?[0]],[0.;4],[0.;4]])
    } else {
        let first=constant(404288371)?;let second=constant(2106384112)?;
        ([first[0],first[1],second[0],second[1]],[constant(4005921460)?[0],constant(523422290)?[0],constant(2569221807)?[0],constant(4266698135)?[0]],[[if specular {constant(211931156)?[0]} else {0.},0.,0.,0.],[0.;4],[0.;4]])
    };
    if water && images.is_some() {
        for (slot,code) in [(1,172),(2,25)] {
            let value = native["code_constants"][code.to_string()].clone();
            if value.is_null() {return Err(format!("Water reveal requires bound native code constant {code}; no initial state is invented").into());}
            special_parms[slot]=serde_json::from_value(value)?;
            if special_parms[slot].iter().any(|value| !value.is_finite()) {return Err("Water reveal effective input is not finite".into());}
        }
    }
    let mut inputs=Vec::new();
    for &(hash,_) in &bindings {
        let binding=native["textures"].as_array().ok_or("Special surface textures are missing")?.iter().find(|binding| binding["hash"] == hash).ok_or("Special surface texture is missing")?;
        let sampler=if water && hash == 3827006327 {10} else {20};
        if binding["sampler"] != sampler {return Err("Special surface source sampler differs from recovered source".into());}
        if let Some(images)=images {
            let name=binding["image"].as_str().ok_or("Special surface image name is missing")?.to_ascii_lowercase();
            inputs.push(Input::read(images.get(&name).ok_or_else(|| format!("Special surface image {name} is missing"))?,sampler)?);
        }
    }
    Ok(Recipe {special:if water {2} else {1},special_parms,inputs,base_specular:specular,geometric_normal:false,second_additive:false,second_normal:0,
        second_reveal:false,second_specular:false,second_parms,third_multiply:false,layer_cutout:false,decal:false,multiply:false,additive:false,
        blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms})
}

fn emissive_recipe(native: &Value, images: Option<&BTreeMap<String,texture::Image>>) -> Result<Recipe> {
    let family=native["technique_set"].as_str().ok_or("Emissive surface family is missing")?;
    let flicker=family == "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3";
    let translucent=family == "mc_sw4_3d_phong_emissive_alcatraz_8q551153";
    let mlv=family == "mlv_sw4_3d_phong_emissive_alcatraz_w1q1w09e";
    let fritz=family.contains("phong_emissive_fritz");let multi=family.contains("phong_emissive_script_multi");
    let tomb=family == "mc_sw4_3d_emissive_tomb_qww536q2";
    let masked=matches!(family,"mc_sw4_3d_phong_emissive_fritz_76q5wqjw" | "mc_sw4_3d_phong_emissive_script_multi_j05zz52w");
    let programs=if fritz {if masked {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_fritz_f19fb54c","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_fritz_97e02993"]}
            else {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_fritz_c65881e3","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_fritz_a0271d3c"]}}
        else if multi {if masked {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_script_multi_edc347f5","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_script_multi_bf233da0"]}
            else {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_script_multi_f502551a","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_script_multi_a7e22f4f"]}}
        else if tomb {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_emissive_tomb_88e8a7be","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_emissive_tomb_f126611f"]}
        else if flicker {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_simple_emissive_flicker_bb5863e8","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_simple_emissive_flicker_cc44003b"]}
        else if translucent {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_alcatraz_d217360d","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_alcatraz_498ceddd"]}
        else if mlv {["sw4_3d_phong_emissive_alcatraz_bb68573a","sw4_3d_phong_emissive_alcatraz_4fa0bc6c","sw4_3d_phong_emissive_alcatraz_b79ce21","sw4_3d_phong_emissive_alcatraz_d43b67bc"]}
        else {["sw4_3d_phong_emissive_alcatraz_b7d697d7","sw4_3d_phong_emissive_alcatraz_431e7c81","sw4_3d_phong_emissive_alcatraz_7c70ecc","sw4_3d_phong_emissive_alcatraz_d885a751"]};
    let bindings=if fritz {vec![(2695565377_u32,1281),(2485960688,1536),(2351529600,1026),(1825069840,771),(1938096808,516),(1346219310,261)]}
        else if flicker {vec![(3574481249,1026),(2485960688,769),(878789447,515),(23340576,256)]}
        else {vec![(2695565377,1025),(2485960688,512),(2351529600,770),(1825069840,259)]};
    let constants=if fritz {vec![(14836402_u32,88,4),(1020264075,80,4),(1418232170,104,4),(1418568053,96,8),(1599130352,116,4),(1732624492,112,4),(2044141368,108,4),(2832011523,84,4)]}
        else if multi {vec![(14836402,88,4),(1020264075,80,4),(2158510423,92,4),(2832011523,84,4)]}
        else if tomb {vec![(14836402,88,4),(604090542,92,4),(1020264075,80,4),(2832011523,84,4),(3662839941,96,4)]}
        else if flicker {vec![(14836402,80,4),(23340584,92,4),(23340854,88,4),(1271696416,96,4),(3936631419,84,4)]}
        else {vec![(14836402,88,4),(604090542,92,4),(1020264075,80,4),(2832011523,84,4),(3493630810,96,4),(3662839941,100,4)]};
    let routes=[(0,0),(1,2),(2,5),(3,1),(4,7),(5,6)];
    for (offset,index) in [4_usize,6].into_iter().enumerate() {
        let technique=native["techniques"].as_array().ok_or("Emissive techniques are missing")?.iter().find(|value| value["index"] == index).ok_or("Emissive selected technique is missing")?;
        let passes=technique["passes"].as_array().ok_or("Emissive passes are missing")?;
        let state=native["state_entries"][index].as_u64().ok_or("Emissive selected state is missing")? as usize;
        if passes.len() != 1 || passes[0]["vertex_shader"] != format!("pimp_shader_{}.hlsl",programs[offset*2])
            || passes[0]["pixel_shader"] != format!("pimp_shader_{}.hlsl",programs[offset*2+1])
            || native["state_bits"][state] != if translucent {json!([422084965_u32,12])} else if masked {json!([403869714_u32,13])} else {json!([403867666_u32,13])} {
            return Err("Emissive selected programs or states differ from recovered source".into());
        }
        let actual=passes[0]["vertex_routes"].as_array().ok_or("Emissive vertex routes are missing")?;
        let args=passes[0]["arguments"].as_array().ok_or("Emissive arguments are missing")?;
        if actual.len() != if mlv {6} else {5} || actual.iter().zip(routes).any(|(route,(source,destination))| route["source"] != source || route["destination"] != destination)
            || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
            || bindings.iter().any(|&(hash,location)| !args.iter().any(|arg| arg["type"] == 2 && arg["value"] == hash && arg["location"] == location && arg["buffer"] == 0 && arg["size"] == 1))
            || args.iter().filter(|arg| arg["type"] == 6).count() != constants.len()
            || constants.iter().any(|&(hash,location,size)| !args.iter().any(|arg| arg["type"] == 6 && arg["value"] == hash && arg["location"] == location && arg["buffer"] == 2 && arg["size"] == size)) {
            return Err("Emissive active source bindings or routes differ from recovered source".into());
        }
        if flicker || fritz || multi || tomb {
            let mut inputs=vec![(if flicker {170} else if tomb {172} else {169},1,if flicker {384} else if tomb {416} else {368})];
            if flicker || fritz {inputs.push((25,2,64));}
            for (code,buffer,location) in inputs {
                if !args.iter().any(|arg| arg["type"] == 5 && arg["value"] == 16777216_u32+code && arg["buffer"] == buffer && arg["location"] == location && arg["size"] == 16) {
                    return Err("Emissive script/time bindings differ from recovered source".into());
                }
            }
        }
    }
    let literal=|hash| -> Result<[f32;4]> {
        let value=native["constants"].as_array().ok_or("Emissive authored constants are missing")?.iter().find(|value| value["hash"] == hash).ok_or("Emissive authored constant is missing")?;
        let literal:[f32;4]=serde_json::from_value(value["literal"].clone())?;
        if literal.iter().any(|value| !value.is_finite()) {return Err("Emissive authored constant is not finite".into());}
        Ok(literal)
    };
    let constant=|hash| -> Result<f32> {Ok(literal(hash)?[0])};
    for &(hash,_,_) in &constants {constant(hash)?;}
    let (parms,second_parms)=if fritz {
        let speed=literal(1418568053)?;
        ([constant(1020264075)?,constant(2832011523)?,constant(14836402)?,constant(1418232170)?],[speed[0],speed[1],constant(2044141368)?,constant(1732624492)?])
    } else if multi {([0.,0.,0.,constant(14836402)?],[constant(1020264075)?,constant(2832011523)?,constant(2158510423)?,if masked {1.} else {0.}])}
        else if tomb {([constant(604090542)?,constant(3662839941)?,0.,constant(14836402)?],[constant(1020264075)?,constant(2832011523)?,0.,0.])}
        else if flicker {([constant(14836402)?,constant(3936631419)?,constant(23340854)?,constant(23340584)?],[constant(1271696416)?,0.,0.,0.])}
        else {([constant(604090542)?,constant(3662839941)?,constant(3493630810)?,constant(14836402)?],[constant(1020264075)?,constant(2832011523)?,0.,0.])};
    let mut special_parms:[[f32;4];3]=[[0.;4];3];special_parms[0][0]=if fritz {constant(1599130352)?} else if translucent {1.} else {0.};
    if fritz {special_parms[0][1]=if masked {1.} else {0.};}
    if (fritz || multi || tomb) && images.is_some() {
        let mut inputs=vec![(1,if tomb {"172"} else {"169"})];
        if fritz {inputs.push((2,"25"));}
        for (slot,key) in inputs {
            let value=native["code_constants"][key].clone();
            if value.is_null() {return Err(format!("Emissive surface requires bound native code constant {key}; no initial state is invented").into());}
            special_parms[slot]=serde_json::from_value(value)?;
            if special_parms[slot].iter().any(|value| !value.is_finite()) {return Err("Emissive effective input is not finite".into());}
        }
    }
    let mut inputs=Vec::new();
    for &(hash,_) in &bindings {
        let binding=native["textures"].as_array().ok_or("Emissive textures are missing")?.iter().find(|binding| binding["hash"] == hash).ok_or("Emissive texture is missing")?;
        let sampler=if fritz && hash == 1346219310 {2} else {20};
        if binding["sampler"] != sampler {return Err("Emissive source sampler differs from recovered source".into());}
        if let Some(images)=images {
            let name=binding["image"].as_str().ok_or("Emissive image name is missing")?.to_ascii_lowercase();
            inputs.push(Input::read(images.get(&name).ok_or_else(|| format!("Emissive image {name} is missing"))?,sampler)?);
        }
    }
    Ok(Recipe {special:if fritz {16} else if multi {32} else if tomb {64} else if flicker {8} else {4},special_parms,inputs,base_specular:true,geometric_normal:false,second_additive:false,second_normal:0,
        second_reveal:false,second_specular:false,second_parms,third_multiply:false,layer_cutout:false,decal:false,multiply:false,additive:false,
        blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms})
}

fn tile_period(recipe: &Recipe) -> Result<[usize;2]> {
    let mut period=[1;2];
    for axis in 0..2 {
        period[axis]=(1..=16).find(|&period| [recipe.parms[axis],recipe.parms[axis+2]].into_iter().all(|scale| {
            let scaled=scale as f64*period as f64;scaled.is_finite() && scaled.fract() == 0.
        })).ok_or("Tile surface has no bounded exact repeat period for its authored micro scales")?;
    }
    Ok(period)
}

pub fn prepare_special(native: &Value, images: &BTreeMap<String,texture::Image>, uri: &str, bounds: Option<[[f32;2];2]>, pack: &mut Pack, shared: &mut BTreeMap<String,String>) -> Result<Value> {
    let reviewed=special_recipe(native,None)?;
    let mut sources=BTreeMap::new();
    let active=&native["techniques"].as_array().unwrap().iter().find(|technique| technique["index"] == 4).unwrap()["passes"][0]["arguments"];
    for binding in native["textures"].as_array().unwrap().iter().filter(|binding| active.as_array().unwrap().iter().any(|arg| arg["type"] == 2 && arg["value"] == binding["hash"])) {
        let name=binding["image"].as_str().ok_or("Special surface image name is missing")?.to_ascii_lowercase();
        let path=if let Some(path)=shared.get(&name) {path.clone()} else {
            let image=images.get(&name).ok_or_else(|| format!("Special surface image {name} is missing"))?;
            let path=format!("prepared/material-inputs/{}/rgba.ktx2",shared.len());
            let encoded=texture::encode(image,false,"reflectance")?;texture::validate(&encoded)?;
            if pack.entries.insert(path.clone(),Arc::new(encoded)).is_some() {return Err("Special surface input URI already exists".into());}
            shared.insert(name.clone(),path.clone());path
        };
        sources.insert(binding["hash"].as_u64().unwrap().to_string(),json!({"uri":path,"sampler":binding["sampler"],"transfer":"source_encoded_rgba"}));
    }
    let required:&[&str]=match reviewed.special {2 => &["172","25"],16 => &["169","25"],32 => &["169"],64 => &["172"],_ => &[]};
    if required.iter().any(|key| native["code_constants"][*key].is_null()) {
        let reason=match reviewed.special {
            2 => "Water reveal requires native per-object scriptVector3 and gameTime inputs, wet-normal/refraction sampling, leading-edge emission, separate reflection amount and the linear model color transform; no initial state is invented",
            16 => "Native Fritz material requires per-object scriptVector0.x/y and gameTime.w for source warp, luma, UV-height sparkle, specular and glow evaluation; no initial state is invented",
            32 => "Native threshold emission requires per-object scriptVector0.z and its inclusive authored threshold; no initial state is invented",
            _ => "Native tomb emission requires per-object scriptVector3 RGBA modulation before source RGB-alpha emission multiplication; no initial state is invented",
        };
        return Ok(json!({"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
            "extras":{"surface":{"version":3,"identity":native["name"],"native":native,"channels":[],"source_inputs":sources,
                "source_effective_inputs":required.iter().map(|key| key.parse::<u32>().unwrap()).collect::<Vec<_>>(),"pending_runtime":reason}}}));
    }
    let recipe=special_recipe(native,Some(images))?;
    let (minimum,extent,sampler) = if recipe.special == 1 {
        let period=tile_period(&recipe)?;([0.;2],period.map(|value| value as f32),20)
    } else if matches!(recipe.special,2 | 16) {
        let bounds=bounds.ok_or_else(|| format!("Bound nonperiodic surface {} requires source-derived UV bounds",native["name"]))?;
        if bounds.iter().flatten().any(|value| !value.is_finite() || recipe.special == 2 && *value<0.) || (0..2).any(|axis| bounds[1][axis]<=bounds[0][axis]) {
            return Err(format!("Bound nonperiodic surface {} UV domain {bounds:?} is outside the recovered source domain{}",native["name"],if recipe.special == 2 {": native log2 receives source UV directly and is undefined for negative coordinates"} else {""}).into());
        }
        (bounds[0],std::array::from_fn(|axis| bounds[1][axis]-bounds[0][axis]),116)
    } else {([0.;2],[1.;2],20)};
    let dimensions:[usize;2]=std::array::from_fn(|axis| {
        recipe.inputs.iter().enumerate().filter(|(index,_)| recipe.special != 8 || *index != 3).map(|(index,input)| {
            let scale=if recipe.special == 1 {match index {2 | 4 | 6 => recipe.parms[axis].abs(),3 | 5 | 7 => recipe.parms[axis+2].abs(),_ => 1.}}
                else if recipe.special == 16 {match index {4 => recipe.second_parms[3].abs(),5 => recipe.parms[3].abs(),_ => 1.}} else {1.};
            ((if axis == 0 {input.width} else {input.height}) as f64*scale as f64*extent[axis] as f64).ceil() as usize
        }).max().unwrap_or(0).max(1)
    });
    if dimensions.into_iter().any(|size| size>16_384) {return Err("Special surface bake exceeds supported texture dimensions at source density".into());}
    let [width,height]=dimensions;let count=width.checked_mul(height).ok_or("Special surface dimensions overflow")?;
    let baker=gpu::Baker::new(&recipe)?;
    let mut outputs:[Vec<[f32;4]>;4]=std::array::from_fn(|_| Vec::with_capacity(count));
    for start in (0..count).step_by(262_144) {
        let samples:Vec<gpu::Sample>=(start..(start+262_144).min(count)).map(|index| {
            let uv=[minimum[0]+((index%width) as f32+0.5)/width as f32*extent[0],minimum[1]+((index/width) as f32+0.5)/height as f32*extent[1]];
            let mut sample=[0.;20];sample[..2].copy_from_slice(&uv);sample
        }).collect();
        for pixel in baker.sample(&samples)? {for (output,pixel) in outputs.iter_mut().zip(pixel) {output.push(pixel);}}
    }
    let emission=if recipe.special != 1 {
        let mut emission=vec![[0.;4];count];
        parallel_chunks_mut(&mut emission,4096,|start,pixels| {
            for (offset,pixel) in pixels.iter_mut().enumerate() {
                let index=start+offset;let uv=[minimum[0]+((index%width) as f32+0.5)/width as f32*extent[0],minimum[1]+((index/width) as f32+0.5)/height as f32*extent[1]];
                *pixel=recipe.emission_pixel(uv,outputs[2][index][3]);
            }
            Ok(())
        })?;
        Some(emission)
    } else {None};
    if recipe.special != 1 {for response in &mut outputs[2] {response[3]=1.;}}
    let mut value=store(native,uri,width,height,outputs,sampler,0,pack)?;
    let surface=&mut value["extras"]["surface"];
    surface["standard"]["metallic"]=json!(if recipe.base_specular {1.} else {0.});
    surface["standard"]["reflectance"]=json!(if recipe.base_specular {0.5} else {0.});
    surface["standard"]["uv_transform"]=json!([1./extent[0],0.,0.,1./extent[1],-minimum[0]/extent[0],-minimum[1]/extent[1]]);
    surface["source_vertex_color_transfer"]=json!("gamma2");
    surface["source_inputs"]=json!(sources);
    surface["bake"]["source_uv_min"]=json!(minimum);
    surface["bake"]["source_uv_extent"]=json!(extent);
    if matches!(recipe.special,2 | 16) {
        surface["bake"]["layout"]=json!("bounded_source_uv_domain");
        surface["approximations"].as_array_mut().unwrap().retain(|value| !value.as_str().is_some_and(|value| value.starts_with("Coarse atlas mip")));
        surface["approximations"].as_array_mut().unwrap().push(json!("Nonperiodic source equations are sampled over supplied geometry UV bounds with clamped output; finite texel, edge and baked mip filtering remain approximations"));
    }
    surface["approximations"].as_array_mut().unwrap().push(json!("Vertex gamma-2 transfer is evaluated before interpolation; the native shader squares interpolated primary RGB. These agree for constant per-triangle RGB; varying tint requires geometry-dependent evaluation"));
    if recipe.special == 1 {
        surface["bake"]["layout"]=json!("exact_uv_repeat_period");
        surface["source_macro_occlusion"]=json!({"source_hash":3976875907_u32,"operation":"saturate(red*red+adjust)","adjust":recipe.second_parms[2],
            "consumers":if recipe.base_specular {json!(["indirect_diffuse","direct_specular","reflection_specular"])} else {json!(["indirect_diffuse"])}});
        surface["source_reflection_amount"]=json!(recipe.special_parms[0][0]);
        if recipe.second_parms[2]<1. || recipe.base_specular && recipe.special_parms[0][0] != 1. {
            surface["pending_runtime"]=json!("Native tile macro occlusion affects indirect diffuse and, in specular variants, direct and reflection specular separately; reflection_amount scales only reflection specular. Distinct runtime lighting consumers remain unconnected");
        }
        surface["approximations"].as_array_mut().unwrap().push(json!("All active source maps repeat on the derived integer UV period, including fractional authored micro scales. Source bilinear texels are evaluated before finite baked mip filtering; vertex tint is retained separately with gamma-2 transfer"));
    } else {
        surface["standard"]["emissive"]=json!([1.,1.,1.]);
        let path=format!("{uri}/emissive.ktx2");
        let image=mip_chain(width,height,emission.unwrap(),"color")?;
        let srgb=image.format != 19;
        let encoded=texture::encode(&image,srgb,"emissive")?;texture::validate(&encoded)?;pack.entries.insert(path.clone(),Arc::new(encoded));
        surface["channels"].as_array_mut().unwrap().push(json!({"uri":path,"role":"emissive","layer":0,"color_space":if srgb {"srgb"} else {"linear"},"sampler":sampler}));
        if recipe.special == 2 {
            surface["source_effective_inputs"]=native["code_constants"].clone();
            surface["source_reflection_amount"]=json!(recipe.special_parms[0][3]);
            surface["source_model_color_transform"]=json!({"buffer":0,"rows":[114,115,116],"operation":"linear_diffuse_plus_specular_plus_emission_before_fog"});
            surface["pending_runtime"]=json!("Water reveal prepared for explicitly bound script/time inputs; live per-object updates, wet UV sampling, separate reflection amount and the native linear model color transform remain unconnected");
        } else if recipe.special == 8 {
            let source=&recipe.inputs[3];
            let lookup:Vec<f32>=(0..source.height).map(|row| source.sample([recipe.second_parms[0],(row as f32+0.5)/source.height as f32])[0]).collect();
            if lookup.iter().any(|value| !value.is_finite()) {return Err("Native flicker lookup contains non-finite source texels".into());}
            surface["scalar_animation"]=json!({"time_mode":"native_elapsed","lookup":lookup,"sampler":source.sampler,
                "speed":recipe.parms[1],"minimum":recipe.parms[2],"maximum":recipe.parms[3],"script_vector":1,"script_component":0});
            surface["source_effective_inputs"]=json!([170,25]);
            surface["source_flicker"]=json!({"script_constant":170,"component":0,"time_constant":25,"time_component":3,
                "lookup_hash":23340576,"lookup_column":recipe.second_parms[0],"lookup_time":"game_time_w_times_authored_speed",
                "lookup_rows":source.height,"lookup_transfer":"source_encoded_red","lookup_filter":"native_fixed_column_bilinear_then_vertical_sampler",
                "range":[recipe.parms[2],recipe.parms[3]],"hdr_amount":recipe.parms[0],"hdr_binding":"squared_in_static_emissive_texture",
                "script_binding":{"normal_view_base":[0.,0.,0.,0.],"override":"active_object_mapped_script_vector","restore":"normal_view_base"},
                "time_binding":{"source":"render_client_milliseconds","seconds_multiplier_bits":0x3a83126f_u32,"shader_start":"source_script_model_zero"},
                "operation":"lookup_range_times_saturate_one_minus_script_times_hdr_before_emission_square"});
        } else if matches!(recipe.special,16 | 32 | 64) {
            surface["source_effective_inputs"]=native["code_constants"].clone();
            if recipe.special != 16 {
                surface["source_specular_parameters"]=json!({"reflection_amount":recipe.second_parms[0],"direct_specular_amount":recipe.second_parms[1]});
                if recipe.special == 64 {surface["source_specular_parameters"]["specular_push"]=json!(recipe.parms[0]);surface["source_specular_parameters"]["gloss_push"]=json!(recipe.parms[1]);}
            }
            let masked=if recipe.special == 16 {recipe.special_parms[0][1] != 0.} else {recipe.special == 32 && recipe.second_parms[3] != 0.};
            if recipe.special == 16 {
                surface["source_fritz"]=json!({"script_constant":169,"warp_component":0,"emission_component":1,"time_constant":25,"time_component":3,
                    "warp_scale":recipe.second_parms[3],"warp_amount":recipe.second_parms[2],"sparkle_scale":recipe.parms[3],"sparkle_speed":[recipe.second_parms[0],recipe.second_parms[1]],
                    "luma_boost":recipe.special_parms[0][0],"luma_weights":[0.3,0.59,0.11],"sparkle_vertical":"uv0.y_cubed_times_sparkle_alpha",
                    "glow_mix_base":"unwarped_color_rgb","glow_alpha":"warped_glow_alpha_when_script_x_positive","specular":"warped_luma_mix_preserving_original_gloss",
                    "reflection_amount":recipe.parms[0],"direct_specular_amount":recipe.parms[1],"hdr_amount":recipe.parms[2]});
                surface["pending_runtime"]=json!("Native Fritz surface prepared for explicit scriptVector0.x/y and gameTime.w; source scoped parameter generation and live nonperiodic warp/sparkle material updates remain unconnected");
            } else if recipe.special == 32 {
                surface["source_emission_threshold"]=json!({"script_constant":169,"component":2,"threshold":recipe.second_parms[2],"comparison":"greater_than_or_equal","hdr_amount":recipe.parms[3]});
                surface["pending_runtime"]=json!("Native threshold emission prepared for explicit scriptVector0.z using its inclusive authored threshold; source scoped parameter generation and live emission updates remain unconnected");
            } else {
                surface["source_emission_modulation"]=json!({"script_constant":172,"channels":"rgba","operation":"multiply_glow_rgba_then_rgb_times_alpha_times_hdr_before_square","hdr_amount":recipe.parms[3]});
                surface["pending_runtime"]=json!("Native tomb emission prepared for explicit scriptVector3 RGBA; source scoped parameter generation and live per-channel emission updates remain unconnected");
            }
            if masked {
                surface["presentation"]["alpha"]=json!("mask");surface["presentation"]["cutoff"]=json!(128_f32/255.);
                surface["source_framebuffer_alpha"]=json!("texture_vertex_coverage_after_cutoff");
                let reason=surface["pending_runtime"].as_str().unwrap().to_owned();
                surface["pending_runtime"]=json!(format!("{reason}; the native masked pass also retains framebuffer coverage after its 128/255 cutoff"));
            }
        } else {
            surface["source_emissive"]=json!({"map_hash":1825069840,"push":recipe.parms[2],"hdr_amount":recipe.parms[3],"operation":"square_rgb_times_alpha_times_push_squared_times_hdr"});
            surface["source_specular_parameters"]=json!({"specular_push":recipe.parms[0],"gloss_push":recipe.parms[1],"reflection_amount":recipe.second_parms[0],"direct_specular_amount":recipe.second_parms[1]});
            if recipe.second_parms[..2].iter().any(|value| *value != 1.) {
                surface["pending_runtime"]=json!("Native phong emission is baked, but ReflectionAmount and SpecularAmount scale reflection and direct specular separately; distinct runtime lighting consumers remain unconnected");
            }
            if recipe.special_parms[0][0] != 0. {
                surface["presentation"]["alpha"]=json!("premultiplied");
                surface["source_fragment_alpha"]=json!("sqrt_texture_vertex_coverage");
                surface["pending_runtime"]=json!("Native translucent phong emission uses sqrt texture/vertex framebuffer alpha with coverage-independent specular/emission in gamma2 premultiplied composition; matching runtime composition remains unconnected");
            }
        }
    }
    if native["technique_set"].as_str().is_some_and(|family| family.starts_with("mlv_")) {
        surface["source_vertex_lighting"]=json!({"source":5,"destination":6,"encoded_rgb_scale":32.,"transfer":"gamma2_before_vertex_interpolation",
            "diffuse":{"operation":"divide_by_grid_spherical_harmonics","buffer":3,"rows":[5,6,7],"minimum":0.1},
            "alpha":"sun_shadow_fallback","stage":"separate_indirect_lighting_terms"});
        if recipe.base_specular {surface["source_vertex_lighting"]["specular"]=json!({"operation":"divide_by_reflection_spherical_harmonics","buffer":3,"rows":[8,9,10],"minimum":0.1});}
        surface["pending_runtime"]=json!("Native vertex-lit surface requires secondary RGB squared times32 divided by separate grid/reflection spherical-harmonic responses before interpolation, separate indirect lighting multipliers and secondary-alpha sun-shadow fallback; geometry and runtime consumers remain unconnected independently of material lighting amounts");
    }
    println!("Baked native special surface {}: {width}x{height}, source UV extent {extent:?}",native["name"]);
    Ok(value)
}

fn recipe(native: &Value, images: Option<&BTreeMap<String, texture::Image>>) -> Result<Recipe> {
    let family = native["technique_set"].as_str().ok_or("Layered family is missing")?;
    if special_family(family) { return special_recipe(native,images); }
    let new_programs = match family {
        "lit_sm_r0c0n0s0_b1c1n1" => Some(["c9b52d6f","6278286e","420be797","10572158"]),
        "lit_sm_r0c0n0s0_b1c1n1s1v1" => Some(["c9b52d6f","d03b9b11","420be797","a2149227"]),
        "lit_sm_r0c0n0x0_b1c1n1s1" => Some(["c9b52d6f","aa3a8509","420be797","d8158c3f"]),
        "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2" => Some(["3250349a","675e1822","b9eefe62","15711114"]),
        "lit_sm_r0c0s0_b1c1" => Some(["f55271f1","5e9f74f0","7eecbb09","2cb07dc6"]),
        "lit_sm_r0c0n0x0_a1c1_m2c2" => Some(["8b6985a0","ad25a73a","d74f58","df0aae0c"]),
        "lit_sm_r0c0n0x0_b1c1n1_a2c2" => Some(["3250349a","f5127bac","b9eefe62","873d729a"]),
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2" => Some(["3250349a","4751c8d3","b9eefe62","357ec1e5"]),
        "lit_sm_r0c0n0x0_b1c1_b2c2n2" => Some(["3250349a","76c68e0c","b9eefe62","4e9873a"]),
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" => Some(["94510929","d0c7b3bf","1fefc3d1","a2e8ba89"]),
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" => Some(["94510929","d29e97e0","1fefc3d1","a0b19ed6"]),
        "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" => Some(["94510929","cf9388f8","1fefc3d1","bdbc81ce"]),
        "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" => Some(["94510929","7dd03b87","1fefc3d1","fff32b1"]),
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2" => Some(["3250349a","c6dc192c","b9eefe62","b4f3101a"]),
        "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2" => Some(["94510929","76b69a5","1fefc3d1","75446093"]),
        "lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" => Some(["94510929","60dd249f","1fefc3d1","12f22da9"]),
        "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" => Some(["3250349a","a3337049","b9eefe62","d11c797f"]),
        "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3" => Some(["aaeb8157","b81c762c","21554baf","ca337f1a"]),
        "lit_sm_r0c0n0x0_t1c1n1" => Some(["c9b52d6f","5d72fcf0","420be797","2f5df5c6"]),
        _ => None,
    };
    let third_multiply = family == "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3";
    let layer_cutout = family == "lit_sm_r0c0n0x0_t1c1n1";
    let second_reveal = matches!(family,"lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2");
    let second_specular = family == "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" || second_reveal && family != "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2";
    let first_add_second_multiply = family == "lit_sm_r0c0n0x0_a1c1_m2c2";
    let second_additive = third_multiply || matches!(family,"lit_sm_r0c0n0x0_b1c1n1_a2c2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2");
    let second_normal = if matches!(family,"lit_sm_r0c0n0x0_b1c1_b2c2n2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2") {1} else if second_reveal || family == "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" {2} else {0};
    let base_specular = matches!(family,"lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1" | "lit_sm_r0c0n0s0_b1c1n1s1v1" | "lit_sm_r0c0s0_b1c1");
    let geometric_normal = family == "lit_sm_r0c0s0_b1c1";
    let response_decal = matches!(family,"lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2");
    let simple_layer_specular = family == "lit_sm_r0c0n0x0_b1c1n1s1";
    let diffuse_reveal = native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1v1";
    let double_blend = native["technique_set"] == "lit_sm_r0c0n0x0_b1c1_b2c2";
    let blend_color = second_normal == 1 || geometric_normal || double_blend || native["technique_set"] == "lit_sm_r0c0n0x0_b1c1";
    let additive = first_add_second_multiply || native["technique_set"] == "lit_sm_r0c0n0x0_a1c1";
    let multiply_response = native["technique_set"] == "lit_sm_r0c0n0x0_m1c1";
    let multiply_decal = native["technique_set"] == "lit_sm_r0c0n0_m1c1_m2c2";
    let multiply = multiply_response || multiply_decal || native["technique_set"] == "lit_sm_r0c0n0_m1c1";
    let base_alpha_reveal = third_multiply || response_decal || native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1s1v1";
    let base_alpha_response = layer_cutout || second_additive || second_normal != 0 || simple_layer_specular || diffuse_reveal || blend_color || additive || multiply_response || base_alpha_reveal || native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1";
    let reveal_decal = native["technique_set"] == "lit_sm_r0c0n0_b1c1n1v1_m2c2";
    let decal = first_add_second_multiply || second_additive || second_normal != 0 || response_decal || double_blend || multiply_decal || reveal_decal || native["technique_set"] == "lit_sm_r0c0n0_b1c1n1_m2c2";
    let layer_specular = second_reveal && matches!(family,"lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2") || simple_layer_specular || family == "lit_sm_r0c0n0s0_b1c1n1s1v1" || base_alpha_reveal || native["technique_set"] == "lit_sm_r0c0n0_b1c1n1s1v1";
    let reveal = diffuse_reveal || reveal_decal || layer_specular && !simple_layer_specular || native["technique_set"] == "lit_sm_r0c0n0_b1c1n1v1";
    let selected_programs = new_programs.map(|names| names.map(|name| format!("pimp_shader_layer_lmap_{name}.hlsl")));
    let (vertex, pixel) = if let Some(programs) = &selected_programs { (programs[0].as_str(),programs[1].as_str()) }
        else if diffuse_reveal { ("pimp_shader_layer_lmap_c9b52d6f.hlsl", "pimp_shader_layer_lmap_18793676.hlsl") }
        else if double_blend { ("pimp_shader_layer_lmap_8b6985a0.hlsl", "pimp_shader_layer_lmap_cfff3f36.hlsl") }
        else if blend_color { ("pimp_shader_layer_lmap_9bf34854.hlsl", "pimp_shader_layer_lmap_9f70e132.hlsl") }
        else if additive || multiply_response { ("pimp_shader_layer_lmap_9bf34854.hlsl", if additive {"pimp_shader_layer_lmap_b50e886e.hlsl"} else {"pimp_shader_layer_lmap_254af097.hlsl"}) }
        else if multiply_decal { ("pimp_shader_layer_lmap_e2f44d32.hlsl", "pimp_shader_layer_lmap_61a9e8fe.hlsl") }
        else if multiply { ("pimp_shader_layer_lmap_f26e80c6.hlsl", "pimp_shader_layer_lmap_7982c7aa.hlsl") }
        else if base_alpha_reveal { ("pimp_shader_layer_lmap_c9b52d6f.hlsl", "pimp_shader_layer_lmap_7f753776.hlsl") }
        else if base_alpha_response { ("pimp_shader_layer_lmap_c9b52d6f.hlsl", "pimp_shader_layer_lmap_cd368409.hlsl") }
        else if reveal_decal { ("pimp_shader_layer_lmap_b0b22809.hlsl", "pimp_shader_layer_lmap_5c9a2e1f.hlsl") }
        else if decal { ("pimp_shader_layer_lmap_b0b22809.hlsl", "pimp_shader_layer_lmap_89d59c60.hlsl") }
        else if layer_specular { ("pimp_shader_layer_lmap_c9b52d6f.hlsl", "pimp_shader_layer_lmap_4a20c8d9.hlsl") }
        else if reveal { ("pimp_shader_layer_lmap_a028e5fd.hlsl", "pimp_shader_layer_lmap_44b1014b.hlsl") }
        else { ("pimp_shader_layer_lmap_a028e5fd.hlsl", "pimp_shader_layer_lmap_91feb334.hlsl") };
    let lit = native["techniques"].as_array().unwrap().iter().find(|value| value["index"] == 4).ok_or("Layered lit technique is missing")?;
    let passes = lit["passes"].as_array().ok_or("Layered technique passes are missing")?;
    if passes.len() != 1 || passes[0]["pixel_shader"] != pixel || passes[0]["vertex_shader"] != vertex
    { return Err("Layered material selected programs differ from recovered source".into()); }
    let state = native["state_entries"][4].as_u64().ok_or("Layered lit state is missing")? as usize;
    if native["state_bits"][state] != json!([403867666_u32,13]) { return Err("Layered material has unreviewed render states".into()); }
    if new_programs.is_some() || base_alpha_reveal || diffuse_reveal || multiply || additive || blend_color {
        let sun = native["techniques"].as_array().unwrap().iter().find(|value| value["index"] == 6).ok_or("Layered sun technique is missing")?;
        let passes = sun["passes"].as_array().ok_or("Layered sun passes are missing")?;
        let state = native["state_entries"][6].as_u64().ok_or("Layered sun state is missing")? as usize;
        if passes.len() != 1 || passes[0]["vertex_shader"] != if let Some(programs) = &selected_programs { programs[2].as_str() } else if double_blend { "pimp_shader_layer_lmap_d74f58.hlsl" } else if additive || multiply_response || blend_color { "pimp_shader_layer_lmap_104d82ac.hlsl" } else if multiply_decal { "pimp_shader_layer_lmap_694a87ca.hlsl" } else if multiply { "pimp_shader_layer_lmap_79d04a3e.hlsl" } else { "pimp_shader_layer_lmap_420be797.hlsl" }
            || passes[0]["pixel_shader"] != if let Some(programs) = &selected_programs { programs[3].as_str() } else if diffuse_reveal { "pimp_shader_layer_lmap_6a563f40.hlsl" } else if double_blend { "pimp_shader_layer_lmap_bdd03600.hlsl" } else if blend_color { "pimp_shader_layer_lmap_ed5fe804.hlsl" } else if additive { "pimp_shader_layer_lmap_c7218158.hlsl" } else if multiply_response { "pimp_shader_layer_lmap_5765f9a1.hlsl" } else if multiply_decal { "pimp_shader_layer_lmap_1386e1c8.hlsl" } else if multiply { "pimp_shader_layer_lmap_badce9c.hlsl" } else { "pimp_shader_layer_lmap_d5a3e40.hlsl" }
            || native["state_bits"][state] != json!([403867666_u32,13]) {
            return Err("Layered sun programs or render states differ from recovered source".into());
        }
    }
    let reveal_parms = if reveal {
        if !passes[0]["arguments"].as_array().unwrap().iter().any(|arg| arg["type"] == 6 && arg["value"] == 2294217777_u32 && arg["buffer"] == 1 && arg["location"] == 944 && arg["size"] == 16) {
            return Err("Layered reveal parameter shader binding differs from recovered source".into());
        }
        let constant = native["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 2294217777_u32).ok_or("Layered reveal parameters are missing")?;
        let values: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
        if !values.iter().all(|value| value.is_finite()) || values[0] <= 0. || values[1] < 0. || (!second_reveal && !base_alpha_reveal && !diffuse_reveal && values[1] == 0.) { return Err("Layered reveal parameters are outside the recovered domain".into()); }
        values
    } else { [0.;4] };
    let second_parms = if second_reveal {
        let constant = native["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 2294217778_u32).ok_or("Second-layer reveal parameters are missing")?;
        let values: [f32;4] = serde_json::from_value(constant["literal"].clone())?;
        if !values.iter().all(|value| value.is_finite()) || values[0] <= 0. || values[1] < 0. { return Err("Second-layer reveal parameters are outside the recovered domain".into()); }
        values
    } else {[0.;4]};
    let expected = [(0,0),(1,2),(2,5),(3,1),(4,7),(5,6),(6,8),(8,10),(9,11)];
    let geometric_routes = [(0,0),(1,2),(2,5),(3,1),(5,6),(6,8)];
    let third_routes = [(0,0),(1,2),(2,5),(3,1),(4,7),(5,6),(6,8),(7,9),(8,10)];
    let expected = if third_multiply { &third_routes[..] } else if geometric_normal { &geometric_routes[..] } else if second_normal == 2 { &expected[..] } else if (multiply || additive || blend_color) && second_normal == 0 { &expected[..7] } else { &expected[..8] };
    let routes = passes[0]["vertex_routes"].as_array().ok_or("Layered vertex routes are missing")?;
    if routes.len() != expected.len() || routes.iter().zip(expected.iter().copied()).any(|(route, (source, destination))| route["source"] != source || route["destination"] != destination) {
        return Err("Layered material vertex declaration differs from recovered source".into());
    }
    let mut inputs = Vec::new();
    let mut bindings = if geometric_normal { vec![(2695565377_u32,20,0),(3054311504,20,258)] }
        else if base_specular { vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,515),(2486480606,20,772)] }
        else { vec![(2695565377_u32,20,0),(1507003663,20,257),(3054311504,if multiply {10} else {20},514)] };
    if !base_specular && !multiply && !additive && !blend_color { bindings.push((2486480606,20,if decal {1027} else {771})); }
    if decal { bindings.push((3054311507,if double_blend || second_additive || second_normal != 0 {20} else {10},if multiply || additive || blend_color {771} else if response_decal {773} else {772})); }
    if second_normal != 0 { bindings.push((2486480605,20,if second_normal == 1 {1028} else {1285})); }
    if layer_specular { bindings.push((3532022562,20,if base_specular {1285} else if response_decal {1284} else {1028})); }
    if base_specular { bindings.push((887934131,20,if geometric_normal {513} else {1026})); }
    if third_multiply {
        bindings = vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,514),(2486480606,20,1283),
            (3054311507,20,773),(3532022562,20,1540),(3054311506,74,1030)];
    } else if second_reveal || second_specular {
        bindings = if second_normal == 1 {
            vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,514),(3054311507,20,771),(2486480605,20,1028),(3532022561,20,1285)]
        } else if base_specular {
            let mut bindings = vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,515),(2486480606,20,1028),
                (3054311507,20,if layer_specular {774} else {773}),(2486480605,20,if layer_specular {1287} else {1286})];
            if layer_specular {bindings.push((3532022562,20,1797));}
            bindings.push((3532022561,20,if layer_specular {2056} else {1799}));
            bindings.push((887934131,20,1538));
            bindings
        } else if !layer_specular {
            vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,514),(2486480606,20,1027),
                (3054311507,20,772),(2486480605,20,1285),(3532022561,20,1542)]
        } else {
            let mut bindings = vec![(2695565377,20,0),(1507003663,20,257),(3054311504,20,514),(2486480606,20,1027),
                (3054311507,20,773),(2486480605,20,1286),(3532022562,20,1540)];
            if second_specular {bindings.push((3532022561,20,1799));}
            bindings
        };
    }
    if new_programs.is_some() {
        for index in [4,6] {
            let pass = &native["techniques"].as_array().unwrap().iter().find(|value| value["index"] == index).unwrap()["passes"][0];
            let routes = pass["vertex_routes"].as_array().ok_or("Layered vertex routes are missing")?;
            let args = pass["arguments"].as_array().ok_or("Layered shader arguments are missing")?;
            if routes.len() != expected.len() || routes.iter().zip(expected).any(|(route,&(source,destination))| route["source"] != source || route["destination"] != destination)
                || args.iter().filter(|arg| arg["type"] == 2).count() != bindings.len()
                || bindings.iter().any(|&(hash,_,slot)| !args.iter().any(|arg| arg["type"] == 2 && arg["value"] == hash && arg["location"] == slot))
                || args.iter().filter(|arg| arg["type"] == 6).count() != usize::from(reveal)+usize::from(second_reveal)
                || reveal && !args.iter().any(|arg| arg["type"] == 6 && arg["value"] == 2294217777_u32 && arg["buffer"] == 1 && arg["location"] == 944 && arg["size"] == 16)
                || second_reveal && !args.iter().any(|arg| arg["type"] == 6 && arg["value"] == 2294217778_u32 && arg["buffer"] == 1 && arg["location"] == 960 && arg["size"] == 16) {
                return Err("Layered response inputs differ from recovered source".into());
            }
        }
    }
    for (hash, expected_sampler, location) in bindings {
        let binding = native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash).ok_or("Layered source texture binding is missing")?;
        let sampler = binding["sampler"].as_u64().and_then(|value| u8::try_from(value).ok()).ok_or("Layered sampler is not a byte")?;
        if (sampler != expected_sampler && !(((reveal_decal || response_decal) && hash == 3054311507 || multiply && hash == 3054311504) && sampler == 74))
            || !passes[0]["arguments"].as_array().unwrap().iter().any(|argument| argument["type"] == 2 && argument["value"] == hash && argument["location"] == location) {
            return Err("Layered texture has an unreviewed sampler or shader slot".into());
        }
        let name = binding["image"].as_str().ok_or("Layered source image name is missing")?;
        if let Some(images) = images {
            let image = images.get(&name.to_ascii_lowercase()).ok_or("Layered source image is missing")?;
            println!("Layered input {name}: {}x{}",image.width,image.height);
            inputs.push(Input::read(image,sampler)?);
        }
    }
    Ok(Recipe {special:0,special_parms:[[0.;4];3],third_multiply,layer_cutout,second_reveal,second_specular,second_parms,second_additive,second_normal,base_specular,geometric_normal,inputs,decal,multiply,additive,blend_color,reveal,layer_specular,base_alpha_response,parms:reveal_parms})
}

fn layered(native: &Value, surfaces: &[Surface], recipe: &Recipe, baker: &gpu::Baker, uri: &str, converted: &mut BTreeMap<String,Value>, pack: &mut Pack) -> Result<Vec<Surface>> {
    let (decal,layer_specular,inputs) = (recipe.decal,recipe.has_specular(),&recipe.inputs);
    let mut charts = Vec::<Chart>::new();
    let mut shared_charts = BTreeMap::<Vec<u32>,(usize,[usize;3])>::new();
    let mut uv_area = 0_f64;
    for (surface_index, surface) in surfaces.iter().enumerate() {
        for first in (0..surface.positions.len()).step_by(3) {
            let varying_frame = !constant_frame(surface,first);
            let auxiliary = (0..3).map(|i| recipe.auxiliary(surface, first+i)).collect::<Result<Vec<_>>>()?;
            let layer_uv: Vec<_> = auxiliary.iter().map(|value| value.0).collect();
            let decal_uv: Vec<_> = auxiliary.iter().map(|value| value.1).collect();
            let third_uv: Vec<_> = auxiliary.iter().map(|value| value.4).collect();
            let uv_streams = if recipe.third_multiply { vec![&surface.uv[first..first+3],layer_uv.as_slice(),decal_uv.as_slice(),third_uv.as_slice()] }
                else if decal { vec![&surface.uv[first..first+3],layer_uv.as_slice(),decal_uv.as_slice()] }
                else { vec![&surface.uv[first..first+3],layer_uv.as_slice()] };
            let offsets: Vec<[f32;2]> = uv_streams.iter().enumerate().map(|(stream,uv)| std::array::from_fn(|axis| {
                let clamped = inputs.iter().enumerate().any(|(input,image)| {
                    let source = recipe.input_uv(input);
                    source == stream && image.sampler & (32 << axis) != 0
                });
                if clamped { 0. } else { uv.iter().map(|value| value[axis]).fold(f32::INFINITY,f32::min).floor() }
            })).collect();
            let keys: [Vec<u32>;3] = std::array::from_fn(|i| {
                let mut key = Vec::new();
                for (uv,offset) in uv_streams.iter().zip(&offsets) { for axis in 0..2 { key.push((uv[i][axis]-offset[axis]).to_bits()); } }
                key.push(surface.colors[first+i][1] as u32);
                if decal { key.push(surface.colors[first+i][2] as u32); }
                if recipe.third_multiply { key.push(surface.colors[first+i][3] as u32); }
                key.extend(auxiliary[i].2.map(f32::to_bits));
                key.extend(auxiliary[i].3.map(f32::to_bits));
                key.push(varying_frame as u32);
                if varying_frame {
                    key.extend(surface.normals[first+i].map(f32::to_bits));
                    key.extend(surface.tangents[first+i].map(f32::to_bits));
                    key.push(surface.tangents[first][3].to_bits());
                }
                key
            });
            let mut canonical = [0,1,2];
            canonical.sort_by(|&a,&b| keys[a].cmp(&keys[b]));
            let key: Vec<_> = canonical.iter().flat_map(|&i| keys[i].iter().copied()).collect();
            if let Some(&(chart,original)) = shared_charts.get(&key) {
                let target: &mut ChartTriangle = &mut charts[chart].triangles[0];
                let order = std::array::from_fn(|vertex| canonical[original.iter().position(|&value| value == vertex).unwrap()]);
                target.copies.push((surface_index,first,order));
                continue;
            }
            let coordinates: Vec<[[f32;2];3]> = inputs.iter().enumerate().map(|(input,image)| {
                    let uv = match recipe.input_uv(input) {0 => &surface.uv[first..first+3],2 => &decal_uv,3 => &third_uv,_ => &layer_uv};
                    std::array::from_fn(|i| [uv[i][0]*image.width as f32,uv[i][1]*image.height as f32])
            }).collect();
            let metric = footprint(&coordinates)?;
            let basis = [[0.,0.],[1.,0.],[0.,1.]];
            let edge = |a: usize,b: usize| std::array::from_fn(|axis| basis[b][axis]-basis[a][axis]);
            let distance = |a: usize,b: usize| quadratic(metric,edge(a,b),edge(a,b)).max(0.);
            let order = [[0,1,2],[1,2,0],[2,0,1]].into_iter().max_by(|a,b| distance(a[0],a[1]).total_cmp(&distance(b[0],b[1]))).unwrap();
            let length = distance(order[0],order[1]).sqrt();
            let along = if length > 0. {
                quadratic(metric,edge(order[0],order[1]),edge(order[0],order[2]))/length
            } else { 0. };
            let altitude = (distance(order[0],order[2])-along*along).max(0.).sqrt();
            for uv in &coordinates {
                uv_area += (((uv[1][0]-uv[0][0]) as f64*(uv[2][1]-uv[0][1]) as f64-(uv[1][1]-uv[0][1]) as f64*(uv[2][0]-uv[0][0]) as f64)*0.5).abs();
            }
            let width = (length.ceil() as usize).max(2);
            let height = (altitude.ceil() as usize).max(2);
            let apex = if length > 0. { along/length*width as f64 } else { 0. };
            let mut points = [[0.;2];3];
            for (index,point) in [[0.,0.],[width as f64,0.],[apex,height as f64]].into_iter().enumerate() { points[order[index]]=point; }
            let a: [f64;2] = std::array::from_fn(|axis| points[1][axis]-points[0][axis]);
            let b: [f64;2] = std::array::from_fn(|axis| points[2][axis]-points[0][axis]);
            let destination = [a[0]*a[0]+a[1]*a[1],a[0]*b[0]+a[1]*b[1],b[0]*b[0]+b[1]*b[1]];
            if !dominates(destination,metric) { return Err("Rounded chart reduces a source footprint".into()); }
            shared_charts.insert(key,(charts.len(),canonical));
            charts.push(Chart {triangles:vec![ChartTriangle {surface:surface_index,first,points,metric:destination,copies:vec![(surface_index,first,[0,1,2])]}],width,height,x:0,y:0});
        }
    }
    let charts = merge_islands(charts,surfaces)?;
    let area = charts.iter().try_fold(0_usize, |area, chart| area.checked_add((chart.width+2*GUTTER).checked_mul(chart.height+2*GUTTER)?)).ok_or("Layered atlas size overflow")?;
    let largest = charts.iter().map(|chart| chart.width+2*GUTTER).max().ok_or("Layered material has no triangles")?;
    let core_area: usize = charts.iter().map(|chart| chart.width*chart.height).sum();
    println!("Layered chart allocation {}: {} unique charts for {} triangles, {area} padded texels, {core_area} core texels, {uv_area:.0} input UV triangle texels, longest chart {largest}",native["name"],charts.len(),charts.iter().flat_map(|chart| &chart.triangles).map(|triangle| triangle.copies.len()).sum::<usize>());
    println!("Verified all source directional footprints; rectangle area needs at least {} 8192-square pages before packing slack",area.div_ceil(8192*8192));
    let pages = place_pages(charts)?;
    let mut assigned: Vec<_> = surfaces.iter().map(|surface| vec![false;surface.indices.len()/3]).collect();
    let mut fragments = Vec::new();
    for page in &pages {
        let mut groups = BTreeMap::<usize,Vec<usize>>::new();
        for triangle in page.charts.iter().flat_map(|chart| &chart.triangles) {
            for &(surface,first,_) in &triangle.copies {
                if first%3 != 0 || std::mem::replace(assigned.get_mut(surface).and_then(|triangles| triangles.get_mut(first/3)).ok_or("Page triangle exceeds source surface")?,true) {
                    return Err("Atlas page assignment duplicates or misaligns a source triangle".into());
                }
                groups.entry(surface).or_default().push(first/3);
            }
        }
        fragments.push(groups);
    }
    if assigned.iter().flatten().any(|&assigned| !assigned) { return Err("Atlas page assignment omits source triangles".into()); }
    let texels: usize = pages.iter().map(|page| page.width*page.height).sum();
    println!("Planned {} bounded atlas pages for {}: {texels} allocated texels, {} source-surface/page mesh parts",pages.len(),native["name"],fragments.iter().map(BTreeMap::len).sum::<usize>());
    let mut remapped = surfaces.to_vec();
    let mut result = Vec::new();
    for (page_index,page) in pages.iter().enumerate() {
    let (width,height) = (page.width,page.height);
    println!("Baking atlas page {}/{}: {width}x{height}, {} islands",page_index+1,pages.len(),page.charts.len());
    let mut albedo = vec![[0.,0.,0.,1.]; width*height];
    let mut normals = vec![[0.,0.,1.,1.]; width*height];
    let mut response = if layer_specular || recipe.base_alpha_response { vec![[1.,1.,0.,1.];width*height] } else { Vec::new() };
    let mut specular = if layer_specular { vec![[0.04,0.04,0.04,0.];width*height] } else { Vec::new() };
    let mut basis_errors = [0_f64;2];
    let mut basis_texels = 0_usize;
    let mut basis_triangles = 0_usize;
    let mut raster = Vec::new();
    for chart in &page.charts {
        for triangle in &chart.triangles {
        let surface = &surfaces[triangle.surface];
        let frame = if constant_frame(surface,triangle.first) { None } else { Some(NormalFrame::read(surface,triangle.first)?) };
        if frame.is_some() { basis_triangles+=triangle.copies.len(); }
        let auxiliary = (0..3).map(|i| recipe.auxiliary(surface,triangle.first+i)).collect::<Result<Vec<_>>>()?;
        let lower: [usize;2] = std::array::from_fn(|axis| triangle.points.iter().map(|point| point[axis].floor().max(0.) as usize).min().unwrap());
        let upper: [usize;2] = std::array::from_fn(|axis| triangle.points.iter().map(|point| point[axis].ceil().max(0.) as usize).max().unwrap()+2*GUTTER);
        let end = [upper[0].min(chart.width+2*GUTTER),upper[1].min(chart.height+2*GUTTER)];
        raster.push((chart,triangle,surface,frame,auxiliary,lower,end));
        }
    }
    for band in (0..height).step_by(128) {
        let end_y = (band+128).min(height);
        let mut rows: Vec<Vec<(usize,usize,[f64;3],gpu::Sample)>> = (band..end_y).step_by(8).map(|_| Vec::new()).collect();
        parallel_chunks_mut(&mut rows, 1, |offset, rows| {
            for (row, jobs) in rows.iter_mut().enumerate() {
                let y0 = band+(offset+row)*8;
                let y1 = (y0+8).min(end_y);
                let mut coverage = vec![f64::INFINITY;(y1-y0)*width];
                for (triangle_index,(chart,triangle,surface,_,auxiliary,lower,end)) in raster.iter().enumerate() {
                    let begin = (chart.y+lower[1]).max(y0);
                    let finish = (chart.y+end[1]).min(y1);
                    for y in begin..finish { for x in lower[0]..end[0] {
                        let point = [x as f64+0.5-GUTTER as f64,(y-chart.y) as f64+0.5-GUTTER as f64];
                        let (weights,distance) = triangle_weights(triangle.points,point)?;
                        if distance > (GUTTER*GUTTER) as f64 { continue; }
                        let local = (y-y0)*width+chart.x+x;
                        if coverage[local] <= distance { continue; }
                        coverage[local]=distance;
                        let interpolate = |values: [f32;3]| values.into_iter().zip(weights).map(|(value,weight)| value as f64*weight).sum::<f64>() as f32;
                        let uv0: [f32;2] = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| surface.uv[triangle.first+i][axis])));
                        let uv1: [f32;2] = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| auxiliary[i].0[axis])));
                        let uv2: [f32;2] = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| auxiliary[i].1[axis])));
                        let transform: [f32;4] = std::array::from_fn(|axis| 2.*interpolate(std::array::from_fn(|i| auxiliary[i].2[axis]))-1.);
                        let second_transform: [f32;4] = std::array::from_fn(|axis| 2.*interpolate(std::array::from_fn(|i| auxiliary[i].3[axis]))-1.);
                        let third_uv: [f32;2] = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| auxiliary[i].4[axis])));
                        let alpha = interpolate(std::array::from_fn(|i| surface.colors[triangle.first+i][3] as f32/255.));
                        let green = interpolate(std::array::from_fn(|i| surface.colors[triangle.first+i][1] as f32/255.));
                        let blue = interpolate(std::array::from_fn(|i| surface.colors[triangle.first+i][2] as f32/255.));
                        jobs.push((y*width+chart.x+x,triangle_index,weights,
                            [uv0[0],uv0[1],uv1[0],uv1[1],uv2[0],uv2[1],green,blue,transform[0],transform[1],transform[2],transform[3],second_transform[0],second_transform[1],second_transform[2],second_transform[3],third_uv[0],third_uv[1],alpha,0.]));
                    }}
                }
            }
            Ok(())
        })?;
        let jobs: Vec<_> = rows.into_iter().flatten().collect();
        let samples: Vec<_> = jobs.iter().map(|job| job.3).collect();
        let mut pixels = baker.sample(&samples)?;
        let statistics = std::sync::Mutex::new(([0_f64;2],0_usize));
        parallel_chunks_mut(&mut pixels, 4096, |offset, pixels| {
            let (mut maximum,mut count) = ([0_f64;2],0_usize);
            for (pixel,job) in pixels.iter_mut().zip(&jobs[offset..]) {
                if let Some(frame) = &raster[job.1].3 {
                    let (normal,errors) = frame.convert(job.2,pixel[1])?;
                    pixel[1]=normal;
                    for axis in 0..2 { maximum[axis]=maximum[axis].max(errors[axis]); }
                    count+=1;
                }
            }
            let mut statistics = statistics.lock().map_err(|_| "Normal conversion worker panicked")?;
            for axis in 0..2 { statistics.0[axis]=statistics.0[axis].max(maximum[axis]); }
            statistics.1+=count;
            Ok(())
        })?;
        let (maximum,count) = statistics.into_inner().map_err(|_| "Normal conversion worker panicked")?;
        for axis in 0..2 { basis_errors[axis]=basis_errors[axis].max(maximum[axis]); }
        basis_texels+=count;
        for (job,pixel) in jobs.iter().zip(pixels) {
            let index = job.0;
            if layer_specular || recipe.base_alpha_response { response[index]=pixel[2]; }
            if layer_specular { specular[index]=pixel[3]; }
            albedo[index]=pixel[0];
            normals[index]=pixel[1];
        }
    }
    for chart in &page.charts {
        for triangle in &chart.triangles {
        for &(surface,first,order) in &triangle.copies {
        let surface = &mut remapped[surface];
        for (index,point) in triangle.points.into_iter().enumerate() {
            if point[0] < -1e-6 || point[1] < -1e-6 || point[0] > chart.width as f64+1e-6 || point[1] > chart.height as f64+1e-6 {
                return Err("Remapped source vertex exceeds its atlas chart".into());
            }
            surface.uv[first+order[index]]=[((chart.x+GUTTER) as f64+point[0]) as f32/width as f32,((chart.y+GUTTER) as f64+point[1]) as f32/height as f32];
        }
        let sign = surface.tangents[first][3];
        for index in first..first+3 { surface.colors[index]=[255;4]; surface.tangents[index][3]=sign; }
        }
        }
        }
    let identity = format!("{}#atlas{page_index}",native["name"].as_str().ok_or("Missing native material identity")?.to_ascii_lowercase());
    let mut material = store(native,&format!("{uri}/{page_index}"),width,height,[albedo,normals,response,specular],116,page.charts.len(),pack)?;
    material["extras"]["surface"]["bake"]["normal_basis"] = json!({"destination":"Solari interpolated normalized N/T; signed cross(N,T)",
        "varying_frame_triangles":basis_triangles,"converted_texel_samples":basis_texels,
        "maximum_precompression_solari_error_degrees":basis_errors[0],"maximum_precompression_raster_error_degrees":basis_errors[1]});
    if basis_triangles > 0 {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Native per-vertex bitangents converted into Solari's reconstructed frame per texel; output handedness is constant per triangle. Raster frame differs on varying-frame triangles, with precompression sampled angular error recorded in bake.normal_basis"));
        println!("Converted {basis_triangles} varying-frame triangles ({basis_texels} texel samples); maximum precompression normal error Solari {:.9} degrees, raster {:.6} degrees",basis_errors[0],basis_errors[1]);
    }
    material["name"]=json!(identity);
    material["extras"]["surface"]["identity"]=json!(identity);
    if page_index == 0 { converted.insert(native["name"].as_str().unwrap().to_ascii_lowercase(),material.clone()); }
    converted.insert(identity.clone(),material);
    for (&surface,triangles) in &fragments[page_index] {
        let mut part = remapped[surface].select_triangles(triangles)?;
        part.material=identity.clone();
        result.push(part);
    }
    println!("Baked atlas page {}/{}: {width}x{height}, {} source-surface parts",page_index+1,pages.len(),fragments[page_index].len());
    }
    for surface in surfaces {
        let count: usize = result.iter().filter(|part| part.source_index == surface.source_index).map(|part| part.indices.len()).sum();
        if count != surface.indices.len() { return Err("Paged bake changed a source surface triangle count".into()); }
    }
    Ok(result)
}

fn store(native: &Value, uri: &str, width: usize, height: usize, outputs: [Vec<[f32;4]>;4], sampler: u8, charts: usize, pack: &mut Pack) -> Result<Value> {
    let base_alpha_response = matches!(native["technique_set"].as_str(),Some("lit_sm_r0c0n0x0_t1c1n1" | "lit_sm_r0c0n0x0_a1c1_m2c2" | "lit_sm_r0c0n0x0_b1c1n1_a2c2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2" | "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" | "lit_sm_r0c0n0x0_b1c1n1" | "lit_sm_r0c0n0x0_m1c1" | "lit_sm_r0c0n0x0_a1c1" | "lit_sm_r0c0n0x0_b1c1" | "lit_sm_r0c0n0x0_b1c1_b2c2" | "lit_sm_r0c0n0x0_b1c1n1v1"));
    let base_alpha_reveal = native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1s1v1";
    let layer_specular = native["technique_set"].as_str().is_some_and(|family| emissive_family(family) || special_family(family) && (family.contains("tile_blend_spec") || family.contains("water_on_surface"))) || matches!(native["technique_set"].as_str(),Some("lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2" | "lit_sm_r0c0n0s0_b1c1n1" | "lit_sm_r0c0n0s0_b1c1n1s1v1" | "lit_sm_r0c0n0x0_b1c1n1s1" | "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2" | "lit_sm_r0c0s0_b1c1" | "lit_sm_r0c0n0_b1c1n1s1v1")) || base_alpha_reveal;
    let [albedo,normals,response,specular] = outputs;
    let mut channels = Vec::new();
    let mut outputs = vec![("color",albedo),("normal",normals)];
    if layer_specular || base_alpha_response { outputs.push(("response",response)); }
    if layer_specular { outputs.push(("native-specular",specular)); }
    let mut native_specular = Value::Null;
    for (role, pixels) in outputs {
        let image = mip_chain(width,height,pixels,role)?;
        let path = format!("{uri}/{role}.ktx2");
        let srgb = role == "color" && image.format != 19;
        let bytes = texture::encode(&image,srgb,if role == "native-specular" { "reflectance" } else { role })?;
        texture::validate(&bytes)?;
        pack.entries.insert(path.clone(),Arc::new(bytes));
        if role == "native-specular" { native_specular=json!({"uri":path,"specular_rgb":"linear","gloss_channel":"alpha"}); }
        else {
            if role == "response" && base_alpha_response {
                native_specular=json!({"uri":path,"gloss_channel":"alpha","specular_rgb":"constant_0.04",
                    "source":"Direct-lobe Fresnel floor; native environment Fresnel fit differs"});
            }
            channels.push(json!({"uri":path,"role":role,"layer":0,"color_space":if srgb {"srgb"} else {"linear"},"sampler":sampler}));
        }
    }
    let mut material = json!({"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":native["name"],"native":native,"channels":channels,
            "standard":{"roughness":1.,"metallic":if layer_specular {1.} else {0.},"reflectance":if layer_specular || base_alpha_response {0.5} else {0.},"base_color":[1.,1.,1.,1.]},
            "native_specular":native_specular,
            "presentation":{"alpha":"opaque","cull":"back","fog":true,"visible":true},
            "bake":{"width":width,"height":height,"charts":charts,"source_texel_density":true,"layout":if sampler == 20 {"periodic"} else {"atlas"}},
            "approximations":["Static material bake at source UV texel density; finite rasterization, BC compression and baked mip filtering", "Source normal slopes normalized before runtime texture filtering", "Source specular/gloss fitted to shared metallic/GGX PBR inputs where present; response mips average the fitted inputs"]}}});
    if native["technique_set"] == "lit_sm_r0c0n0x0_t1c1n1" {
        material["extras"]["surface"]["source_framebuffer_alpha"] = json!("selected_layer_alpha_or_one");
        material["extras"]["surface"]["pending_runtime"] = json!("Native opaque layer selection writes selected layer alpha to the framebuffer; current StandardMaterial opaque output forces alpha to one");
    }
    if sampler != 20 {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Coarse atlas mip levels can mix unrelated charts beyond the eight-pixel gutter"));
    }
    if base_alpha_response {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source effective gloss retained in response alpha and fitted from native exponent exp2(13*a) to GGX perceptual roughness (2/(exp2(13*a)+2))^(1/4). Dielectric F0=0.04 uses the native sun-lobe floor; this cannot reproduce the native environment Fresnel/mip fit, direct-lobe geometry/amplitude or inverse-SH compensation exactly. View-dependent lighting is not baked into albedo or emission"));
    }
    if native["technique_set"] == "lit_sm_r0c0n0x0_m1c1" {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source effective gloss is base color alpha multiplied by the encoded multiply tint luminance dot(tint, [0.3,0.6,0.1]); this effective gloss is retained in response alpha before the shared GGX fit"));
    }
    if base_alpha_reveal {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Source gloss mixes base color alpha and layer specular alpha with the authored reveal weight. Mixed linear RGB specular and encoded gloss retained in native-specular. The shared metallic/GGX fit cannot exactly reproduce the native probe Fresnel/mip response or direct-sun lobe geometry/amplitude; view-dependent lighting is not baked into albedo or emission"));
    }
    Ok(material)
}

pub fn supported(family: &str) -> bool {
    special_family(family) || matches!(family,"lit_sm_r0c0n0x0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2_m3c3" | "lit_sm_r0c0n0x0_t1c1n1" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1_b2c2n2s2v2" | "lit_sm_r0c0n0s0_b1c1n1s1v1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2s2v2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_b2c2n2v2" | "lit_sm_r0c0n0x0_a1c1_m2c2" | "lit_sm_r0c0n0x0_b1c1n1_a2c2" | "lit_sm_r0c0n0x0_b1c1n1s1v1_a2c2" | "lit_sm_r0c0n0x0_b1c1_b2c2n2" | "lit_sm_r0c0n0x0_b1c1n1_b2c2n2" | "lit_sm_r0c0n0s0_b1c1n1" | "lit_sm_r0c0n0s0_b1c1n1s1v1" | "lit_sm_r0c0n0x0_b1c1n1s1" | "lit_sm_r0c0n0x0_b1c1n1s1v1_m2c2" | "lit_sm_r0c0s0_b1c1" | "lit_sm_r0c0n0x0_b1c1n1v1" | "lit_sm_r0c0n0x0_b1c1_b2c2" | "lit_sm_r0c0n0x0_b1c1" | "lit_sm_r0c0n0x0_a1c1" | "lit_sm_r0c0n0x0_m1c1" | "lit_sm_r0c0n0_m1c1_m2c2" | "lit_sm_r0c0n0_m1c1" | "lit_sm_r0c0n0_b1c1n1" | "lit_sm_r0c0n0_b1c1n1_m2c2" | "lit_sm_r0c0n0_b1c1n1s1v1" | "lit_sm_r0c0n0_b1c1n1v1" | "lit_sm_r0c0n0_b1c1n1v1_m2c2" | "lit_sm_r0c0n0x0_b1c1n1" | "lit_sm_r0c0n0x0_b1c1n1s1v1")
}

pub fn world(catalog: &Catalog<'_>, native: &BTreeMap<String, Value>, images: &BTreeMap<String, texture::Image>, converted: &mut BTreeMap<String,Value>, pack: &mut Pack) -> Result<BTreeMap<String, Vec<Surface>>> {
    let needs_geometry=native.values().any(|material| material["technique_set"].as_str().is_some_and(|family| supported(family) && !special_family(family)));
    let mut supplied=Vec::new();
    if needs_geometry {
        for (_,zone,root) in catalog.definitions(17) {supplied.extend(geometry::world(catalog,zone,root)?);}
    }
    surfaces(native,supplied,images,converted,pack)
}

pub fn surfaces(native: &BTreeMap<String,Value>, supplied: Vec<Surface>, images: &BTreeMap<String,texture::Image>, converted: &mut BTreeMap<String,Value>, pack: &mut Pack) -> Result<BTreeMap<String,Vec<Surface>>> {
    let mut required:BTreeMap<_,_>=native.iter().filter(|(_,material)| material["technique_set"].as_str().is_some_and(supported)).collect();
    let special:Vec<String>=required.iter().filter(|(_,material)| material["technique_set"].as_str().is_some_and(special_family)).map(|(name,_)| (*name).clone()).collect();
    let mut shared_sources=BTreeMap::new();
    for key in special {
        let material=required.remove(&key).unwrap();
        let mut bounds=[[f32::INFINITY;2],[f32::NEG_INFINITY;2]];
        let nonperiodic=material["technique_set"] == "mc_sw4_3d_water_on_surface_reveal_77320z68"
            || material["technique_set"].as_str().unwrap().contains("phong_emissive_fritz");
        if nonperiodic && material["code_constants"].is_object() {
            for surface in supplied.iter().filter(|surface| surface.material.eq_ignore_ascii_case(&key)) {
                for uv in &surface.uv {for axis in 0..2 {bounds[0][axis]=bounds[0][axis].min(uv[axis]);bounds[1][axis]=bounds[1][axis].max(uv[axis]);}}
            }
        }
        let bounds=bounds.iter().flatten().all(|value| value.is_finite()).then_some(bounds);
        let uri=format!("prepared/materials/{}/surface",converted.len());
        converted.insert(key.clone(),prepare_special(material,images,&uri,bounds,pack,&mut shared_sources)?);
    }
    if required.is_empty() {return Ok(BTreeMap::new());}
    for material in required.values() {validate_native(material)?;}
    let mut groups=BTreeMap::<String,Vec<Surface>>::new();
    for surface in supplied {
        let key=surface.material.to_ascii_lowercase();
        if required.contains_key(&key) {groups.entry(key).or_default().push(surface.unindex()?);}
    }
    let mut baked = BTreeMap::<String,Vec<Surface>>::new();
    for (key, material) in required {
        let surfaces = groups.remove(key).ok_or_else(|| format!("Layered material {key} has no supplied source geometry"))?;
        let source_counts: BTreeMap<_,_> = surfaces.iter().map(|surface| (surface.name.clone(),surface.indices.len())).collect();
        let input_triangles: usize = surfaces.iter().map(|surface| surface.indices.len()/3).sum();
        let material_recipe = recipe(material,Some(images))?;
        let baker = gpu::Baker::new(&material_recipe)?;
        let mut periodic_cache = BTreeMap::<Vec<u32>,String>::new();
        let mut remaining = Vec::new();
        let mut periodic_triangles = 0;
        for surface in surfaces {
            if (!material_recipe.reveal && !material_recipe.base_alpha_response && !material_recipe.base_specular) || material_recipe.decal
                || material_recipe.inputs.iter().any(|image| image.sampler & 96 != 0) { remaining.push(surface); continue; }
            let mut triangles = BTreeMap::<Vec<u32>,Vec<usize>>::new();
            let mut variable = Vec::new();
            for triangle in 0..surface.indices.len()/3 {
                if let Some(recipe) = periodic_key(&surface,triangle*3,material_recipe.color_only_layer())? { triangles.entry(recipe).or_default().push(triangle); }
                else { variable.push(triangle); }
            }
            for (recipe,triangles) in triangles {
                let identity = if let Some(identity) = periodic_cache.get(&recipe) { identity.clone() } else {
                    let identity = format!("{key}#periodic{}",periodic_cache.len());
                    let uri = format!("prepared/materials/{}/periodic",converted.len());
                    let mut value = periodic(material,&material_recipe,&baker,&recipe,&uri,pack)?;
                    value["name"] = json!(identity);
                    value["extras"]["surface"]["identity"] = json!(identity);
                    converted.insert(identity.clone(),value);
                    periodic_cache.insert(recipe,identity.clone());
                    identity
                };
                let mut part = surface.select_triangles(&triangles)?;
                part.material = identity;
                part.colors.fill([255;4]);
                periodic_triangles += triangles.len();
                baked.entry(surface.name.clone()).or_default().push(part);
            }
            if !variable.is_empty() { remaining.push(surface.select_triangles(&variable)?); }
        }
        let remaining_triangles: usize = remaining.iter().map(|surface| surface.indices.len()/3).sum();
        if periodic_triangles+remaining_triangles != input_triangles { return Err("Layered partition changed the source triangle count".into()); }
        println!("Layered source partition {key}: {periodic_triangles} periodic triangles in {} shared recipes, {remaining_triangles} variable triangles",periodic_cache.len());
        let uri = format!("prepared/materials/{}/layered",converted.len());
        if !remaining.is_empty() {
            for surface in layered(material,&remaining,&material_recipe,&baker,&uri,converted,pack)? { baked.entry(surface.name.clone()).or_default().push(surface); }
        } else {
            let value = converted[periodic_cache.values().next().ok_or("Layered material has no geometry")?].clone();
            converted.insert(key.clone(),value);
        }
        for (name,count) in source_counts {
            if baked.get(&name).ok_or("Baked source surface is missing")?.iter().map(|surface| surface.indices.len()).sum::<usize>() != count {
                return Err(format!("Baked parts change native surface {name} triangle count").into());
            }
        }
    }
    Ok(baked)
}

#[cfg(test)]
fn model_uv_bounds(catalog: &Catalog<'_>, name: &str, nonnegative: bool) -> Result<([[f32;2];2],usize)> {
    let mut bounds=[[f32::INFINITY;2],[f32::NEG_INFINITY;2]];let mut source_triangles=0;
    for (model,zone,root) in catalog.definitions(5) {
        let streams=&catalog.zones[zone].1.streams;
        let materials=crate::fastfile::Address::decode(streams.u32(root.add(36)?)?)?;
        let mut uses=false;
        for index in 0..streams.u8(root.add(6)?)? as usize {
            if catalog.pointer(zone,materials.add(index*4)?,6)?.0.eq_ignore_ascii_case(name) {uses=true;break;}
        }
        if !uses {continue;}
        for lod in geometry::model(catalog,model,zone,root)? {
            for surface in lod.iter().filter(|surface| surface.material.eq_ignore_ascii_case(name)) {
        if nonnegative && surface.uv.iter().flatten().any(|value| *value<0.) {continue;}
        source_triangles+=surface.indices.len()/3;
        for uv in &surface.uv {for axis in 0..2 {bounds[0][axis]=bounds[0][axis].min(uv[axis]);bounds[1][axis]=bounds[1][axis].max(uv[axis]);}}
            }
        }
    }
    if source_triangles == 0 {return Err(format!("Nonperiodic representative {name} model geometry is missing in the requested source UV domain").into());}
    Ok((bounds,source_triangles))
}

#[cfg(test)]
pub fn check_special_response(catalog: &Catalog<'_>, selected: &BTreeMap<String,Value>, images: &BTreeMap<String,texture::Image>, pack: &mut Pack) -> Result<Vec<String>> {
    let native_lookups:std::collections::BTreeSet<_>=selected.values().filter(|native| native["technique_set"] == "mc_sw4_3d_phong_simple_emissive_flicker_9j5f25q3")
        .map(|native| native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == 23340576_u32).unwrap()["image"].as_str().unwrap().to_ascii_lowercase()).collect();
    let images:BTreeMap<_,_>=images.iter().map(|(name,image)| {
        let level=if native_lookups.contains(name) {0} else {(0..image.levels.len()).find(|&level| (image.width>>level).max(1)<=64 && (image.height>>level).max(1)<=64).unwrap_or(image.levels.len()-1)};
        (name.clone(),texture::Image {width:(image.width>>level).max(1),height:(image.height>>level).max(1),depth:(image.depth>>level).max(1),faces:image.faces,format:image.format,flags:image.flags,levels:image.levels[level..].to_vec()})
    }).collect();
    let mut shared=BTreeMap::new();let mut converted=BTreeMap::new();let mut maximum=0_f32;let mut fractional=0;
    for (name,native) in selected {
        validate_native(native)?;
        let uri=format!("prepared/special-check/{}/material",converted.len());
        let value=prepare_special(native,&images,&uri,None,pack,&mut shared)?;
        for input in value["extras"]["surface"]["source_inputs"].as_object().ok_or("Special source inputs were not retained")?.values() {
            texture::validate(pack.get(input["uri"].as_str().ok_or("Special source input URI is missing")?)?)?;
        }
        if native["technique_set"] == "mc_sw4_3d_water_on_surface_reveal_77320z68" {
            if value["extras"]["surface"]["pending_runtime"].is_null() {return Err("Unbound water was silently assigned an initial state".into());}
            let (all_bounds,all_triangles)=model_uv_bounds(catalog,name,false)?;
            let (bounds,source_triangles)=if all_bounds.iter().flatten().any(|value| *value<0.) {
                let mut bound=native.clone();bound["code_constants"]=json!({"172":[0.,0.,0.5,0.],"25":[0.,0.,0.,1.25]});
                let error=prepare_special(&bound,&images,&format!("{uri}/negative-domain"),Some(all_bounds),pack,&mut shared).unwrap_err();
                if !error.to_string().contains("native log2") {return Err(format!("Water {name} negative source domain failed for an unrelated reason: {error}").into());}
                println!("Water {name} retains an unsupported negative-UV source log2 domain across {all_triangles} source triangles/all LODs: {all_bounds:?}");
                model_uv_bounds(catalog,name,true)?
            } else {(all_bounds,all_triangles)};
            for (index,(state,time)) in [(0.2,0.),(0.5,1.25),(0.8,3.)].into_iter().enumerate() {
                let mut bound=native.clone();bound["code_constants"]=json!({"172":[0.,0.,state,0.],"25":[0.,0.,0.,time]});
                let value=prepare_special(&bound,&images,&format!("{uri}/state{index}"),Some(bounds),pack,&mut shared)?;
                if value["extras"]["surface"]["channels"].as_array().unwrap().iter().filter(|channel| channel["role"] == "emissive").count() != 1 {return Err("Bound water leading-edge emission is missing".into());}
                if value["extras"]["surface"]["standard"]["emissive"] != json!([1.,1.,1.]) {return Err(format!("Water {name} emission texture has no neutral StandardMaterial multiplier").into());}
                let recipe=special_recipe(&bound,Some(&images))?;let baker=gpu::Baker::new(&recipe)?;
                let samples:Vec<gpu::Sample>=(0..33).map(|i| {let mut sample=[0.;20];sample[0]=0.125+(i%7) as f32/8.;sample[1]=0.125+(i%5) as f32/6.;sample}).collect();
                for (sample,pixel) in samples.iter().zip(baker.sample(&samples)?) {
                    let expected=recipe.sample_special([sample[0],sample[1]])?;
                    for (actual,expected) in pixel.into_iter().flatten().zip(expected.into_iter().flatten()) {let error=(actual-expected).abs();maximum=maximum.max(error);if error>0.0001 {return Err(format!("Bound water CPU/GPU mismatch: {error}").into());}}
                }
                for channel in value["extras"]["surface"]["channels"].as_array().unwrap() {texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;}
            }
            println!("Checked water effective-input states against {source_triangles} source model triangles with nonnegative UV, bounds {bounds:?}; full geometry and runtime gates retained");
        } else if emissive_family(native["technique_set"].as_str().unwrap()) {
            let kind=special_recipe(native,None)?.special;let controlled=matches!(kind,16 | 32 | 64);
            let bounds=if kind == 16 {let (bounds,triangles)=model_uv_bounds(catalog,name,false)?;println!("Checked Fritz nonperiodic UV domain against {triangles} source triangles/all LODs: {bounds:?}");Some(bounds)} else {None};
            let variants=if kind == 16 {
                [(-0.25,1.),(0.,0.),(0.,1.),(0.0001,1.),(0.5,0.5),(1.,1.),(1.25,1.)].into_iter().enumerate().map(|(index,(warp,amount))| {
                    let mut bound=native.clone();bound["code_constants"]=json!({"169":[warp,amount,0.,0.],"25":[0.,0.,0.,index as f32*1.25]});bound
                }).collect()
            } else if kind == 32 {
                let threshold=native["constants"].as_array().unwrap().iter().find(|constant| constant["hash"] == 2158510423_u32).unwrap()["literal"][0].as_f64().unwrap() as f32;
                [f32::from_bits(threshold.to_bits()-1),threshold,f32::from_bits(threshold.to_bits()+1)].into_iter().map(|state| {
                    let mut bound=native.clone();bound["code_constants"]=json!({"169":[0.,0.,state,0.]});bound
                }).collect()
            } else if kind == 64 {
                [[0.;4],[1.;4],[2.,0.5,1.,0.25],[-1.,1.,-0.5,1.]].into_iter().map(|rgba| {
                    let mut bound=native.clone();bound["code_constants"]=json!({"172":rgba});bound
                }).collect()
            } else {vec![native.clone()]};
            if controlled && value["extras"]["surface"]["pending_runtime"].is_null() {return Err("Scripted emission received an invented initial state".into());}
            for (index,bound) in variants.into_iter().enumerate() {
                let state=if controlled {prepare_special(&bound,&images,&format!("{uri}/state{index}"),bounds,pack,&mut shared)?} else {value.clone()};
                let surface=&state["extras"]["surface"];
                if surface["channels"].as_array().unwrap().iter().filter(|channel| channel["role"] == "emissive").count() != 1 {return Err("Native phong/flicker emission channel is missing".into());}
                if surface["standard"]["emissive"] != json!([1.,1.,1.]) {return Err(format!("Phong/flicker {name} emission texture has no neutral StandardMaterial multiplier").into());}
                let recipe=special_recipe(&bound,Some(&images))?;let baker=gpu::Baker::new(&recipe)?;
                let samples:Vec<gpu::Sample>=(0..33).map(|i| {let mut sample=[0.;20];sample[0]=(i%7) as f32/4.-0.25;sample[1]=(i%5) as f32/3.-0.25;sample}).collect();
                for (sample,pixel) in samples.iter().zip(baker.sample(&samples)?) {
                    let expected=recipe.sample_special([sample[0],sample[1]])?;
                    for (actual,expected) in pixel.into_iter().flatten().zip(expected.into_iter().flatten()) {let error=(actual-expected).abs();maximum=maximum.max(error);if error>0.0001 {return Err(format!("Phong/flicker CPU/GPU mismatch: {error}").into());}}
                }
                for channel in surface["channels"].as_array().unwrap() {texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;}
                texture::validate(pack.get(surface["native_specular"]["uri"].as_str().ok_or("Phong effective specular/gloss is missing")?)?)?;
                if native["technique_set"].as_str().unwrap().starts_with("mlv_") && (surface["source_vertex_lighting"]["source"] != 5 || surface["pending_runtime"].is_null()) {
                    return Err("Neutral phong lighting amounts bypassed required secondary vertex lighting".into());
                }
                if recipe.special == 8 {
                    let animation=&surface["scalar_animation"];
                    if !surface["pending_runtime"].is_null() || animation["time_mode"] != "native_elapsed"
                        || animation["speed"] != json!(recipe.parms[1]) || animation["minimum"] != json!(recipe.parms[2]) || animation["maximum"] != json!(recipe.parms[3])
                        || animation["script_vector"] != 1 || animation["script_component"] != 0 || animation["sampler"] != recipe.inputs[3].sampler
                        || surface["source_flicker"]["script_binding"]["normal_view_base"] != json!([0.,0.,0.,0.]) {
                        return Err(format!("Native flicker {name} live source clock/script binding is missing or changed").into());
                    }
                    let lookup:Vec<f32>=serde_json::from_value(animation["lookup"].clone())?;
                    if lookup.len() != recipe.inputs[3].height {return Err(format!("Native flicker {name} lookup reduced the source row count").into());}
                    let pixels:Vec<_>=lookup.into_iter().map(|value| [value;4]).collect();
                    let period=if recipe.parms[1] == 0. {1.} else {1./recipe.parms[1]};
                    for time in [-1.,0.,1.25,period-0.0001,period,period+0.0001,period*3.+1.25] {
                        let coordinate=time*recipe.parms[1];
                        let original=recipe.inputs[3].sample([recipe.second_parms[0],coordinate])[0];
                        let packed=texture::sample(&pixels,1,pixels.len(),[0.5,coordinate],recipe.inputs[3].sampler)[0];
                        if (original-packed).abs()>0.00001 {return Err(format!("Native flicker {name} fixed-column source sampling changed at elapsed {time}").into());}
                        for script in [-0.5_f32,0.,0.5,1.,1.5] {
                            let factor=(recipe.parms[2]+(recipe.parms[3]-recipe.parms[2])*packed)*(1.-script).clamp(0.,1.);
                            let source=(recipe.parms[2]+(recipe.parms[3]-recipe.parms[2])*original)*(1.-script).clamp(0.,1.)*recipe.parms[0];
                            for uv in [[0.125,0.375],[0.625,0.875]] {
                                let base=recipe.emission_pixel(uv,recipe.parms[0]*recipe.parms[0]);
                                let expected=recipe.emission_pixel(uv,source*source);
                                for channel in 0..3 {
                                    let actual=base[channel]*(factor*factor);
                                    if (actual-expected[channel]).abs()>0.00003*expected[channel].abs().max(1.) {return Err(format!("Native flicker {name} static HDR and live factor differ from source emission").into());}
                                    if script>=1. && actual != 0. {return Err(format!("Native flicker {name} scripted off state retains emission").into());}
                                }
                            }
                        }
                    }
                }
                if recipe.special == 32 {
                    let expected=if index == 0 {0.} else {recipe.parms[3]*recipe.parms[3]};
                    if recipe.sample_special([0.375,0.625])?[2][3] != expected {return Err("Scripted emission threshold is not inclusive at the authored value".into());}
                }
                if controlled && surface["pending_runtime"].is_null() {return Err("Explicit scripted state bypassed its live source-input runtime gate".into());}
                if matches!(recipe.special,16 | 32) && native["state_bits"][native["state_entries"][4].as_u64().unwrap() as usize][0] == 403869714_u32
                    && (surface["presentation"]["alpha"] != "mask" || surface["presentation"]["cutoff"] != json!(128_f32/255.) || surface["source_framebuffer_alpha"].is_null()) {
                    return Err("Native scripted mask lost its coverage/cutoff/framebuffer semantics".into());
                }
            }
        } else {
            let recipe=special_recipe(native,Some(&images))?;let period=tile_period(&recipe)?;
            if period != [1,1] {fractional+=1;}
            if value["extras"]["surface"]["standard"]["uv_transform"] != json!([1./period[0] as f32,0.,0.,1./period[1] as f32,0.,0.]) {return Err("Tile repeat period is not bound to the prepared UV transform".into());}
            let baker=gpu::Baker::new(&recipe)?;
            let coordinates=[[0.125,0.375],[period[0] as f32+0.125,period[1] as f32+0.375],
                [period[0] as f32-1./1024.,period[1] as f32-1./1024.],[period[0] as f32+1./1024.,period[1] as f32+1./1024.]];
            let samples:Vec<gpu::Sample>=coordinates.into_iter().map(|uv| {let mut sample=[0.;20];sample[..2].copy_from_slice(&uv);sample}).collect();
            let pixels=baker.sample(&samples)?;
            for (sample,pixel) in samples.iter().zip(&pixels) {
                let expected=recipe.sample_special([sample[0],sample[1]])?;
                for (&actual,expected) in pixel.iter().flatten().zip(expected.into_iter().flatten()) {let error=(actual-expected).abs();maximum=maximum.max(error);if error>0.00003 {return Err(format!("Tile CPU/GPU mismatch: {error}").into());}}
            }
            if pixels[0].iter().flatten().zip(pixels[1].iter().flatten()).any(|(a,b)| (a-b).abs()>0.00003) {return Err("Authored tile sampling fails its derived repeat-period seam".into());}
            for channel in value["extras"]["surface"]["channels"].as_array().unwrap() {texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?;}
            if recipe.base_specular {texture::validate(pack.get(value["extras"]["surface"]["native_specular"]["uri"].as_str().ok_or("Tile native response is missing")?)?)?;}
            if native["technique_set"].as_str().unwrap().starts_with("mlv_") {
                let mut neutral=native.clone();
                for constant in neutral["constants"].as_array_mut().unwrap() {
                    if constant["hash"] == 2569221807_u32 || constant["hash"] == 211931156_u32 {constant["literal"][0]=json!(1.);}
                }
                let neutral=prepare_special(&neutral,&images,&format!("{uri}/neutral-lighting"),None,pack,&mut shared)?;
                let surface=&neutral["extras"]["surface"];
                if surface["source_vertex_lighting"]["source"] != 5 || surface["source_vertex_lighting"]["alpha"] != "sun_shadow_fallback"
                    || surface["pending_runtime"].as_str().is_none_or(|reason| !reason.contains("secondary RGB")) {
                    return Err("Neutral tile occlusion/reflection incorrectly bypassed required secondary vertex lighting".into());
                }
            }
        }
        converted.insert(name.clone(),value);
    }
    if fractional == 0 {return Err("Special representative batch omitted the fractional tile-period edge".into());}
    println!("Checked {} special representatives at existing source mips <=64px, full native uniform flicker lookups, {fractional} fractional-repeat edges and three explicitly bound water states; maximum CPU/GPU error {maximum}; {} raw input encodings shared",selected.len(),shared.len());
    Ok(crate::material::missing(selected,&converted))
}

#[cfg(test)]
pub fn check_response_geometry(catalog: &Catalog<'_>, materials: &BTreeMap<String,Value>, images: &BTreeMap<String,texture::Image>, pack: &mut Pack) -> Result<Vec<String>> {
    let recipes = materials.iter().map(|(name,native)| Ok((name.clone(),recipe(native,Some(images))?))).collect::<Result<BTreeMap<_,_>>>()?;
    let mut selected = BTreeMap::<String,(usize,Surface)>::new();
    let mut triangles = 0;
    let mut vertices = 0;
    for (_,zone,root) in catalog.definitions(17) {
        for surface in geometry::world(catalog,zone,root)? {
            let key = surface.material.to_ascii_lowercase();
            let Some(recipe) = recipes.get(&key) else { continue; };
            let surface = surface.unindex()?;
            for index in 0..surface.positions.len() { recipe.auxiliary(&surface,index)?; vertices+=1; }
            triangles+=surface.indices.len()/3;
            for triangle in 0..surface.indices.len()/3 {
                let first = triangle*3;
                let spread = (1..3).map(|i| surface.colors[first+i][1].abs_diff(surface.colors[first][1]) as usize
                    + surface.colors[first+i][2].abs_diff(surface.colors[first][2]) as usize).sum::<usize>();
                if selected.get(&key).is_none_or(|&(score,_)| spread>score) {
                    selected.insert(key.clone(),(spread,surface.select_triangles(&[triangle])?));
                }
            }
        }
    }
    if selected.len() != materials.len() { return Err("Representative layered geometry is missing".into()); }
    let mut converted = BTreeMap::new();
    let mut surfaces = Vec::new();
    for (name,(_,surface)) in selected {
        let recipe = &recipes[&name];
        let baker = gpu::Baker::new(recipe)?;
        let uri = format!("prepared/materials/{}/representative",converted.len());
        surfaces.extend(layered(&materials[&name],&[surface],recipe,&baker,&uri,&mut converted,pack)?);
    }
    let mut scene = crate::gltf::Scene::new("layered-response");
    let indices = scene.materials(surfaces.iter(),&converted)?;
    let mesh = scene.mesh("layered-response",surfaces.iter(),&indices,false)?;
    scene.node(json!({"name":"layered-response","mesh":mesh}));
    scene.write("prepared/material.gltf",pack)?;
    for material in converted.values() {
        for channel in material["extras"]["surface"]["channels"].as_array().unwrap() { texture::validate(pack.get(channel["uri"].as_str().unwrap())?)?; }
        texture::validate(pack.get(material["extras"]["surface"]["native_specular"]["uri"].as_str().ok_or("Native specular channel is missing")?)?)?;
    }
    println!("Validated {vertices} representative-material source vertices and {triangles} triangles; baked/exported {} source triangles with KTX2 and native-specular bindings",surfaces.iter().map(|surface| surface.indices.len()/3).sum::<usize>());
    Ok(crate::material::missing(materials,&converted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_emission_preserves_fritz_warp_alpha_glow_base_and_inclusive_thresholds() {
        let input=|pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe=Recipe {special:16,special_parms:[[0.5,1.,0.,0.],[0.,0.5,0.,0.],[0.;4]],
            inputs:vec![Input {width:2,height:1,sampler:20,pixels:vec![[0.2,0.4,0.6,0.2],[0.8,0.6,0.4,0.8]]},input([0.5,0.5,1.,1.]),
                Input {width:2,height:1,sampler:20,pixels:vec![[0.2,0.3,0.4,0.7],[0.8,0.6,0.4,0.1]]},
                Input {width:2,height:1,sampler:20,pixels:vec![[0.9,0.8,0.7,0.25],[0.1,0.2,0.3,0.75]]},input([1.,0.5,0.,0.]),input([0.8,0.6,0.4,0.5])],
            base_specular:true,geometric_normal:false,second_additive:false,second_normal:0,second_reveal:false,second_specular:false,
            second_parms:[0.,0.05,0.5,1.],third_multiply:false,layer_cutout:false,decal:false,multiply:false,additive:false,blend_color:false,
            reveal:false,layer_specular:false,base_alpha_response:false,parms:[1.,1.,2.,4.]};
        recipe.inputs[5].sampler=2;
        let sample=[0.25,0.5,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.];
        let base=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        assert_eq!(base[0][3],0.2);assert_eq!(base[3][3],0.7);
        let emission=recipe.emission_pixel([0.25,0.5],base[2][3]);
        for (actual,expected) in emission.into_iter().zip([0.050625,0.04,0.030625,1.]) {assert!((actual-expected).abs()<0.00001);}
        recipe.special_parms[1][0]=0.5;
        let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        assert_eq!(actual[0][3],0.8);assert_eq!(actual[3][3],0.7);
        let expected=recipe.sample_special([0.25,0.5]).unwrap();
        for (a,b) in actual.into_iter().flatten().zip(expected.into_iter().flatten()) {assert!((a-b).abs()<0.00003);}
        let luma=0.638_f32*0.638*0.5;
        let glow:[f32;3]=std::array::from_fn(|axis| {let base=[0.2,0.4,0.6][axis];let target=luma+0.0625*([0.8,0.6,0.4][axis]-luma);base+0.5*(target-base)});
        let emission=recipe.emission_pixel([0.25,0.5],actual[2][3]);
        for axis in 0..3 {assert!((emission[axis]-(glow[axis]*0.75).powi(2)).abs()<0.00001);}
        recipe.special_parms[0][1]=0.;
        assert_eq!(gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0][0][3],1.);
        recipe.special=32;recipe.parms=[0.,0.,0.,2.];recipe.second_parms=[1.,1.,0.7,1.];
        for (state,weight) in [(f32::from_bits(0.7_f32.to_bits()-1),0.),(0.7,4.),(f32::from_bits(0.7_f32.to_bits()+1),4.)] {
            recipe.special_parms[1][2]=state;
            let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
            assert_eq!(actual[2][3],weight);assert_eq!(actual[0][3],0.2);
        }
        recipe.special=64;recipe.parms=[1.,1.,0.,3.];recipe.special_parms[1]=[2.,0.5,1.,0.25];recipe.inputs[3]=input([0.8,0.6,0.4,0.5]);
        let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];assert_eq!(actual[2][3],9.);
        for (actual,expected) in recipe.emission_pixel([0.25,0.5],actual[2][3]).into_iter().zip([0.36,0.01265625,0.0225,1.]) {assert!((actual-expected).abs()<0.00001);}
    }

    #[test]
    fn phong_emission_push_and_flicker_controls_preserve_source_amplitude_and_hdr_response() {
        let input=|pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe=Recipe {special:4,special_parms:[[0.;4];3],inputs:vec![input([0.2,0.4,0.6,0.25]),input([0.5,0.5,1.,1.]),input([0.4,0.5,0.6,0.8]),input([0.8,0.6,0.4,0.25])],
            base_specular:true,geometric_normal:false,second_additive:false,second_normal:0,second_reveal:false,second_specular:false,second_parms:[1.,1.,0.,0.],
            third_multiply:false,layer_cutout:false,decal:false,multiply:false,additive:false,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms:[0.5,1.25,2.,3.]};
        let sample=[0.375,0.625,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.];
        let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        assert!((actual[2][3]-144.).abs()<0.00001);
        for (actual,expected) in actual[3].into_iter().zip([0.04,0.0625,0.09,1.]) {assert!((actual-expected).abs()<0.00001);}
        assert_eq!(actual[0][3],1.);
        recipe.special_parms[0][0]=1.;
        assert_eq!(gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0][0][3],0.25);
        let image=mip_chain(1,1,vec![[1.5,0.2,0.3,1.24]],"native-specular").unwrap();
        assert_eq!(image.format,19);
        let pixel=texture::pixels(&image,0).unwrap()[0];
        assert_eq!(pixel[0],1.5);assert!((pixel[3]-1.24).abs()<0.001);
        texture::validate(&texture::encode(&image,false,"reflectance").unwrap()).unwrap();
        recipe.special=8;recipe.parms=[3.,0.1,0.2,0.8];recipe.second_parms=[0.3,0.,0.,0.];
        recipe.inputs[3]=input([0.25;4]);
        let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        assert_eq!(actual[2][3],9.);
        assert_eq!(actual[3],[0.04,0.04,0.04,0.25]);
        let expected=recipe.sample_special([sample[0],sample[1]]).unwrap();
        for (a,b) in actual.into_iter().flatten().zip(expected.into_iter().flatten()) {assert!((a-b).abs()<0.00003);}
        for (actual,expected) in recipe.emission_pixel([0.375,0.625],9.).into_iter().zip([0.9216,1.44,2.0736,1.]) {assert!((actual-expected).abs()<0.00001);}
    }

    #[test]
    fn special_surface_gpu_preserves_macro_mask_response_channels_and_bound_water_states() {
        let input=|pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe=Recipe {special:1,special_parms:[[0.;4];3],inputs:vec![input([0.1,0.5,0.9,0.]),input([0.6,0.2,1.,1.]),
            input([0.8,0.3,1.,1.]),input([0.2,0.9,1.,1.]),input([0.2,0.4,0.6,0.]),input([0.8,0.2,0.4,1.]),
            input([0.2,0.4,0.5,0.6]),input([0.8,0.8,0.3,0.2])],base_specular:true,geometric_normal:false,second_additive:false,second_normal:0,
            second_reveal:false,second_specular:false,second_parms:[2.,3.,-0.25,4.],third_multiply:false,layer_cutout:false,decal:false,multiply:false,
            additive:false,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms:[3.,3.,2.5,2.5]};
        assert_eq!(tile_period(&recipe).unwrap(),[2,2]);
        let sample=[0.375,0.625,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.,0.];
        let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
        for (a,b) in actual.into_iter().flatten().zip(recipe.sample_special([sample[0],sample[1]]).unwrap().into_iter().flatten()) {assert!((a-b).abs()<0.00003);}
        for (actual,expected) in actual[3].into_iter().zip([0.25,0.2025,0.25,0.35]) {assert!((actual-expected).abs()<0.00001);}
        recipe.parms[0]=f32::from_bits(0x3f800001);
        assert!(tile_period(&recipe).is_err());
        recipe.special=2;recipe.parms=[0.125,0.5,0.6,1.2];recipe.second_parms=[6.,8.,8.,2.];
        recipe.special_parms=[[4.,4.,4.,2.5],[0.,0.,0.5,0.],[0.,0.,0.,1.25]];
        recipe.inputs=vec![input([0.5,0.,0.,0.]),input([0.6,0.4,1.,1.]),input([1.;4]),input([0.8,0.8,1.,1.]),input([0.4,0.5,0.6,1.]),input([0.2,0.3,0.4,0.25])];
        for positive in [false,true] {
            recipe.inputs[3].pixels[0]=if positive {[0.8,0.8,1.,1.]} else {[0.2,0.2,1.,1.]};
            for state in [0.2,0.5,0.8] {
                recipe.special_parms[1][2]=state;
                let actual=gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap()[0];
                let expected=recipe.sample_special([sample[0],sample[1]]).unwrap();
                for (a,b) in actual.into_iter().flatten().zip(expected.into_iter().flatten()) {assert!((a-b).abs()<0.0001);}
                if state == 0.5 {
                    assert!((actual[2][3]-0.0625).abs()<0.00001);
                    assert!((actual[3][3]-if positive {0.25} else {0.625}).abs()<0.00001);
                } else {assert_eq!(actual[2][3],0.);}
            }
        }
    }

    #[test]
    fn gpu_matches_existing_material_equations_and_sampler_modes() {
        let modes = [(false,false,false,false,false,false,false,false,false,false,0,false,false,false,false),(true,false,false,false,false,false,false,false,false,false,0,false,false,false,false),
            (false,false,false,false,true,false,false,false,false,false,0,false,false,false,false),(true,false,false,false,true,false,false,false,false,false,0,false,false,false,false),
            (false,false,false,false,true,true,false,false,false,false,0,false,false,false,false),(false,false,false,false,true,true,true,false,false,false,0,false,false,false,false),
            (false,false,false,false,false,false,true,false,false,false,0,false,false,false,false),(false,false,false,false,true,false,true,false,false,false,0,false,false,false,false),
            (false,true,false,false,false,false,false,false,false,false,0,false,false,false,false),(true,true,false,false,false,false,false,false,false,false,0,false,false,false,false),
            (false,true,false,false,false,false,true,false,false,false,0,false,false,false,false),(false,false,true,false,false,false,true,false,false,false,0,false,false,false,false),
            (false,false,false,true,false,false,true,false,false,false,0,false,false,false,false),(true,false,false,true,false,false,true,false,false,false,0,false,false,false,false),
            (false,false,false,false,false,false,false,true,false,false,0,false,false,false,false),
            (false,false,false,false,true,true,false,true,false,false,0,false,false,false,false),
            (false,false,false,false,false,true,true,false,false,false,0,false,false,false,false),
            (true,false,false,false,true,true,true,false,false,false,0,false,false,false,false),
            (false,false,false,true,false,false,true,true,true,false,0,false,false,false,false),
            (true,false,true,false,false,false,true,false,false,false,0,false,false,false,false),
            (true,false,false,false,false,false,true,false,false,true,0,false,false,false,false),
            (true,false,false,false,true,true,true,false,false,true,0,false,false,false,false),
            (true,false,false,true,false,false,true,false,false,false,1,false,false,false,false),
            (true,false,false,false,false,false,true,false,false,false,2,false,false,false,false),
            (true,false,false,false,true,true,true,false,false,false,2,true,true,false,false),
            (true,false,false,false,false,false,false,true,false,false,2,true,true,false,false),
            (true,false,false,false,true,true,false,true,false,false,2,true,true,false,false),
            (true,false,false,true,false,false,true,false,false,false,1,true,true,false,false),
            (true,false,false,false,true,true,true,false,false,false,2,true,false,false,false),
            (true,false,false,false,false,false,true,false,false,false,2,true,true,false,false),
            (true,false,false,true,false,false,true,false,false,false,1,false,true,false,false),
            (true,false,false,false,true,true,true,false,false,true,0,false,false,true,false),
            (false,false,false,false,false,false,true,false,false,false,0,false,false,false,true)];
        let mut maximum = 0_f32;
        for (mode,&(decal,multiply,additive,blend_color,reveal,layer_specular,base_alpha_response,base_specular,geometric_normal,second_additive,second_normal,second_reveal,second_specular,third_multiply,layer_cutout)) in modes.iter().enumerate() {
            for sampler in [1,20,33,65,97,42,74,106] {
                let inputs = (0..9).map(|input| Input {width:3,height:5,sampler,
                    pixels:(0..15).map(|index| std::array::from_fn(|channel| ((index*7+input*11+channel*3)%31) as f32/31.)).collect()}).collect();
                let recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply,layer_cutout,second_reveal,second_specular,second_parms:[0.3,if mode%2 == 0 {0.} else {0.625},0.,0.],second_additive,second_normal,base_specular,geometric_normal,inputs,decal,multiply,additive,blend_color,reveal,layer_specular,base_alpha_response,parms:[0.5,1.3,0.,0.]};
                let samples: Vec<gpu::Sample> = (0..513).map(|index| {
                    let u = (index%23) as f32/7.-1.5;
                    let v = (index%17) as f32/6.-0.5;
                    [u,v,u+0.31,v-0.27,u-0.61,v+0.43,(index%13) as f32/12.,(index%11) as f32/10.,0.,-1.,1.,0.,0.6,0.8,-0.8,0.6,u+0.73,v-0.81,(index%7) as f32/6.,0.]
                }).collect();
                let baker = gpu::Baker::new(&recipe).unwrap();
                let actual = baker.sample(&samples).unwrap();
                for (sample,actual) in samples.iter().zip(actual) {
                    let expected = recipe.sample_frames([sample[0],sample[1]],[sample[2],sample[3]],[sample[4],sample[5]],sample[6],sample[7],sample[8..12].try_into().unwrap(),sample[12..16].try_into().unwrap(),sample[16..20].try_into().unwrap()).unwrap();
                    for (channel,(actual,expected)) in actual.into_iter().flatten().zip(expected.into_iter().flatten()).enumerate() {
                        let error = (actual-expected).abs();
                        maximum=maximum.max(error);
                        assert!(error <= 0.00003,"mode {mode}, sampler {sampler}, channel {channel}: GPU {actual}, CPU {expected}, sample {sample:?}");
                    }
                }
            }
        }
        println!("GPU parity: {} recipes, 8 samplers, 513 samples each; maximum absolute channel error {maximum}",modes.len());
        let input = |value| Input {width:1,height:1,sampler:20,pixels:vec![value]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.4,0.7,0.2,0.3]),input([0.5;4]),input([0.8,0.2,0.4,0.7]),input([0.6;4]),input([0.7;4])],
            decal:false,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:true,base_alpha_response:true,parms:[0.5,1.3,0.,0.]};
        let baker = gpu::Baker::new(&recipe).unwrap();
        let sample = [0.25,0.5,0.25,0.5,0.25,0.5,0.6,0.,1.,0.,0.,1.,1.,0.,0.,1.,0.,0.,0.,0.];
        let expected = baker.sample(&[sample]).unwrap()[0];
        let samples = vec![sample;262_147];
        let begin = std::time::Instant::now();
        let actual = baker.sample(&samples).unwrap();
        let gpu_time = begin.elapsed();
        assert_eq!(actual.len(),samples.len());
        assert!(actual.into_iter().all(|pixel| pixel == expected));
        let begin = std::time::Instant::now();
        for sample in &samples {
            std::hint::black_box(recipe.sample([sample[0],sample[1]],[sample[2],sample[3]],[sample[4],sample[5]],sample[6],sample[7],sample[8..12].try_into().unwrap()).unwrap());
        }
        println!("262147 material samples including GPU transfer/readback: GPU {gpu_time:?}, serial CPU {:?}",begin.elapsed());
        recipe.parms=[0.;4];
        assert!(gpu::Baker::new(&recipe).unwrap().sample(&[sample]).unwrap_err().to_string().contains("thresholds"));
    }

    #[test]
    fn authored_specular_preserves_uv0_and_decal_gloss_order() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:true,geometric_normal:false,
            inputs:vec![input([0.6,0.5,0.4,0.1]),input([0.5,0.5,1.,1.]),input([0.2,0.3,0.4,1.]),input([0.5,0.5,1.,1.]),
                input([0.4,0.6,0.8,0.9]),input([0.8,0.6,0.4,0.3])],
            decal:false,multiply:false,additive:false,blend_color:false,reveal:false,layer_specular:true,base_alpha_response:false,parms:[0.;4]};
        assert_eq!((0..6).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,1,1,0]);
        let mixed = recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.,[1.,0.,0.,1.]).unwrap();
        for (actual,expected) in mixed[3].into_iter().zip([0.36,0.36,0.36,0.6]) { assert!((actual-expected).abs()<0.000001); }
        recipe.inputs.pop();
        recipe.base_specular=false;
        recipe.base_alpha_response=true;
        recipe.decal=true;
        recipe.inputs.insert(4,input([0.2,0.4,0.6,0.]));
        let no_decal = recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.,[1.,0.,0.,1.]).unwrap();
        let decal = recipe.sample([0.;2],[0.;2],[0.;2],0.5,1.,[1.,0.,0.,1.]).unwrap();
        assert_eq!(&no_decal[3][..3],&decal[3][..3]);
        assert!((decal[3][3]-0.5*(0.2*0.3+0.4*0.6+0.6*0.1)).abs()<0.000001);
        assert_eq!(no_decal[1],decal[1]);
        assert_eq!((0..6).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,1,2,1]);
        recipe.inputs=vec![input([0.2,0.4,0.6,0.1]),input([0.8,0.6,0.4,0.5]),input([0.4,0.6,0.8,0.7])];
        recipe.geometric_normal=true;
        recipe.base_specular=true;
        recipe.layer_specular=false;
        recipe.blend_color=true;
        recipe.decal=false;
        let geometric = recipe.sample([0.;2],[0.;2],[0.;2],1.,1.,[1.,-1.,-1.,1.]).unwrap();
        assert_eq!(geometric[1],[0.,0.,1.,1.]);
        assert!((geometric[3][3]-0.7).abs()<0.000001);
        assert_eq!((0..3).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,1,0]);
    }

    #[test]
    fn add_multiply_and_second_normal_follow_source_order() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,
            inputs:vec![input([0.4,0.5,0.6,0.8]),input([0.5,0.5,1.,1.]),input([0.8,0.6,0.4,0.5]),input([0.2,0.4,0.6,0.3])],
            decal:true,multiply:false,additive:true,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        let first = recipe.sample([0.;2],[0.;2],[0.;2],0.5,1.,[1.,0.,0.,1.]).unwrap();
        for (actual,expected) in first[0][..3].iter().zip([0.0144,0.0676,0.1764]) { assert!((actual-expected).abs()<0.000001); }
        assert!((first[2][3]-0.288).abs()<0.000001);
        recipe.additive=false;
        recipe.second_additive=true;
        recipe.inputs.insert(3,input([0.5,0.5,1.,1.]));
        let without = recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.,[1.,0.,0.,1.]).unwrap();
        let added = recipe.sample([0.;2],[0.;2],[0.;2],0.5,1.,[1.,0.,0.,1.]).unwrap();
        for (actual,expected) in added[0][..3].iter().zip([0.3136,0.416025,0.5329]) { assert!((actual-expected).abs()<0.000001); }
        assert_eq!(without[1],added[1]);
        assert_eq!(without[2],added[2]);
        recipe.layer_specular=true;
        recipe.inputs.push(input([0.4,0.6,0.8,0.2]));
        let without = recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.,[1.,0.,0.,1.]).unwrap();
        let added = recipe.sample([0.;2],[0.;2],[0.;2],0.5,1.,[1.,0.,0.,1.]).unwrap();
        assert_eq!(without[3],added[3]);
        assert!((added[3][3]-0.65).abs()<0.000001);
        let slope = |x,y| input([(x-f32::from_bits(0xc0010204))/f32::from_bits(0x40808102),(y-f32::from_bits(0xc0010204))/f32::from_bits(0x40808102),1.,1.]);
        recipe.second_additive=false;
        recipe.second_normal=2;
        recipe.layer_specular=false;
        recipe.inputs=vec![input([0.4,0.5,0.6,0.8]),slope(1.,0.),input([0.8,0.6,0.4,1.]),slope(0.,1.),input([0.2,0.4,0.6,1.]),slope(0.5,-0.5)];
        let normal = recipe.sample_frames([0.;2],[0.;2],[0.;2],0.5,0.5,[0.,-1.,1.,0.],[1.,0.,0.,1.],[0.;4]).unwrap()[1];
        let length = 1.125_f32.sqrt();
        for (actual,expected) in normal.into_iter().zip([0.25/length,-0.25/length,1./length,1.]) { assert!((actual-expected).abs()<0.000001); }
        assert_eq!((0..6).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,1,2,2]);
        recipe.second_normal=1;
        recipe.blend_color=true;
        recipe.inputs.remove(3);
        let only_second = recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.5,[1.,0.,0.,1.]).unwrap();
        let length = 1.625_f32.sqrt();
        for (actual,expected) in only_second[1].into_iter().zip([0.75/length,-0.25/length,1./length,1.]) { assert!((actual-expected).abs()<0.000001); }
        assert_eq!(only_second[2][3],0.8);
        assert_eq!((0..5).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,2,2]);
    }

    #[test]
    fn second_reveal_preserves_ordered_encoded_specular_and_independent_gloss() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:true,second_specular:true,second_parms:[0.25,0.,0.,0.],
            second_additive:false,second_normal:2,base_specular:false,geometric_normal:false,
            inputs:vec![input([0.2,0.4,0.6,0.3]),input([0.5,0.5,1.,1.]),input([0.6,0.8,0.4,0.4]),input([0.5,0.5,1.,1.]),
                input([0.8,0.2,0.4,0.5]),input([0.5,0.5,1.,1.]),input([0.8,0.4,0.6,0.7]),input([0.2,0.6,0.4,0.9])],
            decal:true,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:true,base_alpha_response:true,parms:[0.5,0.,0.,0.]};
        let sample = |recipe: &Recipe,blue| recipe.sample([0.;2],[0.;2],[0.;2],0.5,blue,[1.,0.,0.,1.]).unwrap();
        let actual = sample(&recipe,0.5);
        for (actual,expected) in actual[3].into_iter().zip([0.1024,0.1936,0.1444,0.68]) {assert!((actual-expected).abs()<0.000001);}
        let (base,metallic,roughness) = crate::material::pbr_pixel([0.3364,0.1444,0.2116],[0.1024,0.1936,0.1444],0.68);
        for (actual,expected) in actual[0][..3].iter().zip(base) {assert!((actual-expected).abs()<0.000001);}
        assert!((actual[2][1]-roughness).abs()<0.000001);
        assert!((actual[2][2]-metallic).abs()<0.000001);
        assert_eq!((0..8).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,1,2,2,1,2]);
        recipe.second_specular=false;
        recipe.inputs.pop();
        let low = sample(&recipe,0.);
        let high = sample(&recipe,1.);
        assert_eq!(low[3],high[3]);
        assert!((high[3][3]-0.46).abs()<0.000001);
        assert_ne!(low[0],high[0]);
        recipe.second_parms=[0.;4];
        assert!(recipe.sample([0.;2],[0.;2],[0.;2],0.5,0.5,[1.,0.,0.,1.]).is_err());
        let baker = gpu::Baker::new(&recipe).unwrap();
        assert!(baker.sample(&[[0.,0.,0.,0.,0.,0.,0.5,0.5,1.,0.,0.,1.,1.,0.,0.,1.,0.,0.,0.,0.]]).unwrap_err().to_string().contains("thresholds"));
    }

    #[test]
    fn third_multiply_uses_vertex_alpha_and_layer_selection_uses_inclusive_half() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:true,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],
            second_additive:true,second_normal:0,base_specular:false,geometric_normal:false,
            inputs:vec![input([0.2,0.4,0.6,0.3]),input([0.5,0.5,1.,1.]),input([0.6,0.8,0.4,0.4]),input([0.75,0.25,1.,1.]),
                input([0.8,0.2,0.4,0.5]),input([0.8,0.4,0.6,0.7]),input([0.2,0.6,0.4,0.9])],
            decal:true,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:true,base_alpha_response:true,parms:[0.5,0.,0.,0.]};
        let actual = recipe.sample_frames([0.;2],[0.;2],[0.;2],0.5,0.5,[1.,0.,0.,1.],[1.,0.,0.,1.],[0.2,0.3,0.5,0.]).unwrap();
        for (actual,expected) in actual[3].into_iter().zip([0.1936,0.0784,0.1296,0.3358]) { assert!((actual-expected).abs()<0.000001); }
        let (base,_,_) = crate::material::pbr_pixel([0.112896,0.238144,0.188356],[0.1936,0.0784,0.1296],0.3358);
        for (actual,expected) in actual[0][..3].iter().zip(base) {assert!((actual-expected).abs()<0.000001);}
        assert_eq!((0..7).map(|index| recipe.input_uv(index)).collect::<Vec<_>>(),[0,0,1,1,2,1,3]);
        let without = recipe.sample_frames([0.;2],[0.;2],[0.;2],0.5,0.5,[1.,0.,0.,1.],[1.,0.,0.,1.],[0.2,0.3,0.,0.]).unwrap();
        assert_eq!(&actual[3][..3],&without[3][..3]);
        assert!((without[3][3]-0.46).abs()<0.000001);
        recipe.third_multiply=false;
        recipe.layer_cutout=true;
        recipe.second_additive=false;
        recipe.decal=false;
        recipe.reveal=false;
        recipe.layer_specular=false;
        recipe.inputs.truncate(4);
        recipe.inputs[2].pixels[0][3]=0.8;
        let low = recipe.sample([0.;2],[0.;2],[0.;2],0.62499,1.,[1.,0.,0.,1.]).unwrap();
        let high = recipe.sample([0.;2],[0.;2],[0.;2],0.625,1.,[1.,0.,0.,1.]).unwrap();
        for (actual,expected) in low[0].into_iter().zip([0.04,0.16,0.36,1.]) {assert!((actual-expected).abs()<0.000001);}
        for (actual,expected) in high[0].into_iter().zip([0.36,0.64,0.16,0.8]) {assert!((actual-expected).abs()<0.000001);}
        assert_eq!(low[2][3],0.3);
        assert_eq!(high[2][3],0.3);
        assert_ne!(low[1],high[1]);
        let samples = [[0.,0.,0.,0.,0.,0.,0.62499,1.,1.,0.,0.,1.,1.,0.,0.,1.,0.,0.,0.,0.],
            [0.,0.,0.,0.,0.,0.,0.625,1.,1.,0.,0.,1.,1.,0.,0.,1.,0.,0.,0.,0.]];
        let actual = gpu::Baker::new(&recipe).unwrap().sample(&samples).unwrap();
        assert_eq!(actual[0][0][3],1.);
        assert_eq!(actual[1][0][3],0.8);
    }

    #[test]
    fn varying_native_frame_converts_into_solari_basis() {
        for sign in [-1.,1.] {
            let normals = [[0.,0.,1.],[0.,0.6,0.8],[0.6,0.,0.8]];
            let tangents = [[1.,0.,0.],[1.,0.,0.],[0.8,0.,-0.6]];
            let frame = NormalFrame {normals,tangents,bitangents:std::array::from_fn(|i| cross(normals[i],tangents[i]).map(|v| v*sign)),sign};
            let sample = [0.3,-0.4,(0.75_f32).sqrt(),1.];
            for weights in [[1.,0.,0.],[0.,1.,0.],[0.2,0.3,0.5]] {
                let (converted,errors) = frame.convert(weights,sample).unwrap();
                assert!(converted[2] > 0.);
                assert!(errors[0] < 0.00001);
                let interpolate = |values: [[f64;3];3]| std::array::from_fn(|axis| (0..3).map(|i| weights[i]*values[i][axis]).sum::<f64>());
                let n = unit(interpolate(normals)).unwrap();
                let t = unit(interpolate(tangents)).unwrap();
                let b = cross(n,t).map(|v| v*sign);
                let reconstructed_z = (1.-(converted[0] as f64).powi(2)-(converted[1] as f64).powi(2)).sqrt();
                let actual = unit(std::array::from_fn(|axis| t[axis]*converted[0] as f64+b[axis]*converted[1] as f64+n[axis]*reconstructed_z)).unwrap();
                let expected = unit(std::array::from_fn(|axis| (0..3).map(|i| weights[i]*(tangents[i][axis]*sample[0] as f64+frame.bitangents[i][axis]*sample[1] as f64+normals[i][axis]*sample[2] as f64)).sum())).unwrap();
                assert!((0..3).all(|axis| (actual[axis]-expected[axis]).abs() < 0.000001));
                if weights == [0.2,0.3,0.5] { assert!(errors[1] > 0.01); }
            }
        }
        let singular = NormalFrame {normals:[[0.,0.,1.];3],tangents:[[0.,0.,1.];3],bitangents:[[0.;3];3],sign:1.};
        assert!(singular.convert([1.,0.,0.],[0.,0.,1.,1.]).is_err());
    }

    #[test]
    fn mixed_handedness_is_baked_into_a_consistent_output_frame() {
        let frame = NormalFrame {normals:[[0.,0.,1.];3],tangents:[[1.,0.,0.];3],
            bitangents:[[0.,1.,0.],[0.,-1.,0.],[0.,1.,0.]],sign:1.};
        let (middle,errors) = frame.convert([0.5,0.5,0.],[0.,0.8,0.6,1.]).unwrap();
        assert_eq!(middle,[0.,0.,1.,1.]);
        assert_eq!(errors,[0.;2]);
        let (second,errors) = frame.convert([0.,1.,0.],[0.,0.8,0.6,1.]).unwrap();
        assert!((second[1]+0.8).abs() < 0.000001 && (second[2]-0.6).abs() < 0.000001);
        assert!(errors.into_iter().all(|error| error < 0.00001));
    }

    #[test]
    fn multiply_layer_preserves_opaque_coverage_base_normal_and_transfer_order() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.]),input([0.75,0.25,1.,1.]),input([0.5,0.25,1.,0.])],
            decal:false,multiply:true,additive:false,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms:[0.;4]};
        let sample = |recipe: &Recipe,green| recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[0.,-1.,1.,0.]).unwrap();
        let normal = sample(&recipe,0.)[1];
        for (green,expected) in [(0.,[0.64,0.36,0.16]),(0.5,[0.36,0.140625,0.16]),(1.,[0.16,0.0225,0.16])] {
            let actual = sample(&recipe,green);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[1],normal);
        }
        let original = sample(&recipe,0.5);
        recipe.inputs[0].pixels[0][3]=1.;
        recipe.inputs[2].pixels[0][3]=1.;
        assert_eq!(sample(&recipe,0.5),original);
    }

    #[test]
    fn second_multiply_layer_uses_blue_and_its_own_uv_before_transfer() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.]),input([0.75,0.25,1.,1.]),input([0.5,0.25,1.,0.]),
            Input {width:2,height:1,sampler:10,pixels:vec![[0.25,1.,0.5,0.],[1.;4]]}],
            decal:true,multiply:true,additive:false,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:false,parms:[0.;4]};
        let sample = |recipe: &Recipe,green,blue,uv2| recipe.sample([0.;2],[0.;2],uv2,green,blue,[1.,0.,0.,1.]).unwrap();
        let normal = sample(&recipe,0.,0.,[0.25,0.5])[1];
        for (green,blue,expected) in [(0.,1.,[0.04,0.36,0.04]),(1.,0.,[0.16,0.0225,0.16]),
            (1.,1.,[0.01,0.0225,0.04]),(0.5,0.5,[0.140625,0.140625,0.09])] {
            let actual = sample(&recipe,green,blue,[0.25,0.5]);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[1],normal);
        }
        assert_eq!(sample(&recipe,1.,1.,[0.75,0.5]),sample(&recipe,1.,0.,[0.25,0.5]));
        let original = sample(&recipe,0.5,0.5,[0.25,0.5]);
        recipe.inputs[2].pixels[0][3]=1.;
        recipe.inputs[3].pixels[0][3]=1.;
        assert_eq!(sample(&recipe,0.5,0.5,[0.25,0.5]),original);
    }

    #[test]
    fn multiply_gloss_uses_encoded_tint_luminance_and_base_alpha() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.8]),input([0.75,0.25,1.,1.]),input([0.5,0.25,1.,0.])],
            decal:false,multiply:true,additive:false,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        let sample = |recipe: &Recipe,green| recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
        let mut previous_roughness = 0.;
        for (green,gloss) in [(0.,0.8),(0.5,0.56),(1.,0.32)] {
            let actual = sample(&recipe,green);
            assert!((actual[2][3]-gloss).abs()<0.000001);
            assert!(actual[2][1]>previous_roughness);
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[2][2],0.);
            previous_roughness=actual[2][1];
        }
        let original = sample(&recipe,1.);
        recipe.inputs[2].pixels[0][3]=1.;
        assert_eq!(sample(&recipe,1.),original);
        recipe.inputs[0].pixels[0][3]=0.;
        let no_gloss = sample(&recipe,1.);
        assert_eq!(no_gloss[0],original[0]);
        assert_eq!(no_gloss[1],original[1]);
        assert_eq!(no_gloss[2][3],0.);
        assert!((no_gloss[2][1]-0.903602).abs()<0.000001);
    }

    #[test]
    fn oversized_chart_rotation_preserves_geometry_and_directional_metric() {
        let points = [[0.,0.],[11521.,0.],[5760.,64.]];
        let a: [f64;2] = std::array::from_fn(|axis| points[1][axis]-points[0][axis]);
        let b: [f64;2] = std::array::from_fn(|axis| points[2][axis]-points[0][axis]);
        let metric = [a[0]*a[0]+a[1]*a[1],a[0]*b[0]+a[1]*b[1],b[0]*b[0]+b[1]*b[1]];
        let triangle = ChartTriangle {surface:1,first:12,points,metric,copies:vec![(1,12,[0,1,2])]};
        let pages = place_pages(vec![Chart {triangles:vec![triangle],width:11521,height:64,x:0,y:0}]).unwrap();
        assert_eq!(pages.len(),1);
        let chart = &pages[0].charts[0];
        assert!(chart.width+2*GUTTER<=8192 && chart.height+2*GUTTER<=8192);
        let triangle = &chart.triangles[0];
        assert_eq!(triangle.copies,vec![(1,12,[0,1,2])]);
        let a: [f64;2] = std::array::from_fn(|axis| triangle.points[1][axis]-triangle.points[0][axis]);
        let b: [f64;2] = std::array::from_fn(|axis| triangle.points[2][axis]-triangle.points[0][axis]);
        let rotated = [a[0]*a[0]+a[1]*a[1],a[0]*b[0]+a[1]*b[1],b[0]*b[0]+b[1]*b[1]];
        assert!(dominates(rotated,metric) && dominates(metric,rotated));
        let impossible = ChartTriangle {surface:0,first:0,points:[[0.,0.],[20000.,0.],[0.,20000.]],metric:[1.,0.,1.],copies:vec![(0,0,[0,1,2])]};
        assert!(place_pages(vec![Chart {triangles:vec![impossible],width:20000,height:20000,x:0,y:0}]).is_err());
    }

    #[test]
    fn additive_layer_uses_alpha_green_and_preserves_base_gloss() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.75]),input([0.75,0.25,1.,1.]),input([0.8,0.4,0.2,0.5])],
            decal:false,multiply:false,additive:true,blend_color:false,reveal:false,layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        let sample = |recipe: &Recipe,green| recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
        let original = sample(&recipe,0.);
        for (green,expected) in [(0.,[0.64,0.36,0.16]),(0.5,[1.,0.49,0.2025]),(1.,[1.44,0.64,0.25])] {
            let actual = sample(&recipe,green);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[1],original[1]);
            assert_eq!(actual[2],original[2]);
            assert_eq!(actual[2][3],0.75);
        }
        recipe.inputs[2].pixels[0][3]=0.;
        assert_eq!(sample(&recipe,1.),original);
    }

    #[test]
    fn hdr_bake_preserves_linear_color_mips_and_runtime_color_space() {
        let mut pixels = vec![[0.5,0.25,0.,1.];16];
        pixels[0]=[2.5,0.25,0.,1.];
        let image = mip_chain(4,4,pixels.clone(),"color").unwrap();
        assert_eq!(image.format,19);
        assert_eq!(texture::pixels(&image,0).unwrap(),pixels);
        assert_eq!(texture::pixels(&image,1).unwrap()[0],[1.,0.25,0.,1.]);
        assert_eq!(texture::pixels(&image,2).unwrap()[0],[0.625,0.25,0.,1.]);
        let mut pack = Pack::default();
        let native = json!({"name":"additive","technique_set":"lit_sm_r0c0n0x0_a1c1"});
        let material = store(&native,"prepared/test",4,4,[pixels,vec![[0.,0.,1.,1.];16],vec![[1.,0.5,0.,0.75];16],Vec::new()],10,1,&mut pack).unwrap();
        let color = &material["extras"]["surface"]["channels"][0];
        assert_eq!(color["color_space"],"linear");
        assert_eq!(texture::validate(pack.get(color["uri"].as_str().unwrap()).unwrap()).unwrap()["vk_format"],97);
        assert!(mip_chain(0,1,Vec::new(),"color").is_err());
        assert!(mip_chain(2,2,vec![[1.;4]],"color").is_err());
        for value in [f32::NAN,f32::INFINITY,-0.1,70000.] {
            assert!(mip_chain(1,1,vec![[value,0.,0.,1.]],"color").is_err());
        }
        assert!(mip_chain(1,1,vec![[2.,0.,0.,1.1]],"color").is_err());
    }

    #[test]
    fn color_blend_uses_layer_alpha_green_and_preserves_base_normal_gloss() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.75]),input([0.75,0.25,1.,1.]),input([0.2,0.4,0.8,0.5])],
            decal:false,multiply:false,additive:false,blend_color:true,reveal:false,layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        let sample = |recipe: &Recipe,green| recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[0.,-1.,1.,0.]).unwrap();
        let original = sample(&recipe,0.);
        for (green,expected) in [(0.,[0.64,0.36,0.16]),(0.5,[0.4225,0.3025,0.25]),(1.,[0.25,0.25,0.36])] {
            let actual = sample(&recipe,green);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[1],original[1]);
            assert_eq!(actual[2],original[2]);
            assert_eq!(actual[2][3],0.75);
        }
        recipe.inputs[2].pixels[0][3]=0.;
        assert_eq!(sample(&recipe,1.),original);
        recipe.inputs[2].pixels[0][3]=1.;
        let covered = sample(&recipe,1.);
        for (value,expected) in covered[0][..3].iter().zip([0.04,0.16,0.64]) { assert!((value-expected).abs()<0.000001); }
        assert_eq!(covered[2],original[2]);
    }

    #[test]
    fn second_color_blend_uses_blue_alpha_uv2_and_ordered_encoded_composition() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.8,0.6,0.4,0.75]),input([0.75,0.25,1.,1.]),input([0.2,0.4,0.8,0.5]),
            Input {width:2,height:1,sampler:20,pixels:vec![[0.6,0.2,0.4,0.5],[1.,1.,1.,0.]]}],
            decal:true,multiply:false,additive:false,blend_color:true,reveal:false,layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        let sample = |recipe: &Recipe,green,blue,uv2| recipe.sample([0.;2],[0.;2],uv2,green,blue,[1.,0.,0.,1.]).unwrap();
        let original = sample(&recipe,0.,0.,[0.25,0.5]);
        for (green,blue,expected) in [(0.,1.,[0.49,0.16,0.16]),(1.,0.,[0.25,0.25,0.36]),
            (1.,1.,[0.3025,0.1225,0.25]),(0.5,0.5,[0.40640625,0.21390625,0.225625])] {
            let actual = sample(&recipe,green,blue,[0.25,0.5]);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[1],original[1]);
            assert_eq!(actual[2],original[2]);
        }
        assert_eq!(sample(&recipe,1.,1.,[0.75,0.5]),sample(&recipe,1.,0.,[0.25,0.5]));
        recipe.inputs[3].pixels[0][3]=0.;
        assert_eq!(sample(&recipe,1.,1.,[0.25,0.5]),sample(&recipe,1.,0.,[0.25,0.5]));
    }

    #[test]
    fn diffuse_reveal_zero_exponent_blends_normals_without_replacing_base_gloss() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.2,0.4,0.6,0.75]),input([0.5,0.5,1.,1.]),
            input([0.8,0.6,0.2,0.5]),input([0.75,0.25,1.,1.])],
            decal:false,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:false,base_alpha_response:true,parms:[0.25,0.,0.,0.]};
        let sample = |green| recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
        let original = sample(0.);
        for (green,expected) in [(0.,[0.04,0.16,0.36]),(0.5,[0.25,0.25,0.16]),(1.,[0.64,0.36,0.04])] {
            let actual = sample(green);
            for (value,expected) in actual[0][..3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
            assert_eq!(actual[2],original[2]);
            assert_eq!(actual[2][3],0.75);
            assert!((actual[1][..3].iter().map(|v| v*v).sum::<f32>()-1.).abs()<0.000001);
        }
        assert!(sample(1.)[1][0]>sample(0.)[1][0]);
        assert!(sample(1.)[1][1]<sample(0.)[1][1]);
    }

    #[test]
    fn base_alpha_reveal_preserves_mixed_native_gloss_and_zero_exponent() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.2,0.4,0.6,0.75]),input([0.5,0.5,1.,1.]),
            input([0.8,0.6,0.2,0.5]),input([0.5,0.5,1.,1.]),input([0.6,0.4,0.2,0.25])],
            decal:false,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:true,base_alpha_response:true,parms:[0.5,0.5,0.,0.]};
        for (green,expected) in [(0.,[0.04,0.04,0.04,0.75]),(0.5,[0.16,0.09,0.04,0.5]),(1.,[0.36,0.16,0.04,0.25])] {
            let actual = recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
            for (value,expected) in actual[3].iter().zip(expected) { assert!((value-expected).abs()<0.000001); }
            assert_eq!(actual[0][3],1.);
        }
        recipe.parms=[1.,0.,0.,0.];
        let reference=recipe.sample([0.;2],[0.;2],[0.;2],0.,1.,[1.,0.,0.,1.]).unwrap();
        for green in [0.,0.5,1.] {
            assert_eq!(recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap(),reference);
        }
        assert!((reference[3][3]-0.5).abs()<0.000001);
    }

    #[test]
    fn layered_base_alpha_response_survives_complete_layer_coverage() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.2,0.4,1.]),input([0.5,0.5,1.,1.])],decal:false,multiply:false,additive:false,blend_color:false,reveal:false,
            layer_specular:false,base_alpha_response:true,parms:[0.;4]};
        for (alpha,roughness) in [(0.,0.903602),(1.,0.12499237)] {
            recipe.inputs[0].pixels[0][3]=alpha;
            for green in [0.,0.5,1.] {
                let actual=recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
                assert!((actual[2][1]-roughness).abs()<0.000001);
                assert_eq!(actual[2][2],0.);
                assert_eq!(actual[2][3],alpha);
                assert_eq!(actual[0][3],1.);
                if green == 1. {
                    for (value,expected) in actual[0][..3].iter().zip([0.64,0.04,0.16]) {
                        assert!((value-expected).abs()<0.000001);
                    }
                }
            }
        }
    }

    #[test]
    fn reveal_decal_preserves_uv2_axis_addressing_and_encoded_composition() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.2,0.4,0.5]),input([0.75,0.25,1.,1.]),
            Input {width:2,height:2,sampler:74,pixels:vec![[0.9,0.8,0.7,1.],[0.1,0.2,0.3,1.],
                [0.5,0.25,0.75,0.],[1.,0.5,0.25,0.]]}],
            decal:true,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:false,base_alpha_response:false,parms:[0.0713300034403801,0.5,0.,0.]};
        let sample = |recipe: &Recipe, blue| recipe.sample([0.;2],[0.;2],[1.25,1.25],0.5,blue,[0.,-1.,1.,0.]).unwrap();
        let actual=sample(&recipe,0.5);
        for (value,expected) in actual[0][..3].iter().zip([0.140625,0.03515625,0.19140625]) {
            assert!((value-expected).abs()<0.000001);
        }
        assert_eq!(actual[1],sample(&recipe,0.)[1]);
        assert_eq!(actual[2],[1.,1.,0.,1.]);
        assert_eq!(actual[3],[0.;4]);
        for pixel in &mut recipe.inputs[4].pixels { pixel[3]=1.-pixel[3]; }
        assert_eq!(actual,sample(&recipe,0.5));
        recipe.inputs[4].sampler=10;
        assert_ne!(actual[0],sample(&recipe,0.5)[0]);
    }

    #[test]
    fn diffuse_reveal_uses_four_inputs_and_green_only() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let recipe = Recipe {special:0,special_parms:[[0.;4];3],third_multiply:false,layer_cutout:false,second_reveal:false,second_specular:false,second_parms:[0.;4],second_additive:false,second_normal:0,base_specular:false,geometric_normal:false,inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.6,0.2,0.5]),input([0.5,0.5,1.,1.])],
            decal:false,multiply:false,additive:false,blend_color:false,reveal:true,layer_specular:false,base_alpha_response:false,parms:[0.0713300034403801,0.5,0.,0.]};
        for (green,expected) in [(0.,[0.04,0.16,0.36]),(0.5,[0.25,0.25,0.16]),(1.,[0.64,0.36,0.04])] {
            let low = recipe.sample([0.;2],[0.;2],[0.;2],green,0.,[1.,0.,0.,1.]).unwrap();
            let high = recipe.sample([0.;2],[0.;2],[0.;2],green,1.,[1.,0.,0.,1.]).unwrap();
            for (actual,expected) in low[0][..3].iter().copied().zip(expected) { assert!((actual-expected).abs() < 0.000001); }
            assert_eq!(low,high);
            assert_eq!(low[2],[1.,1.,0.,1.]);
            assert_eq!(low[3],[0.;4]);
        }
    }
}
