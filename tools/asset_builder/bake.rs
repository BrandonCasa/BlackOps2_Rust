use crate::{convert::Catalog, core::*, geometry::{self, Surface}, pack::Pack, texture};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

const GUTTER: usize = 8;

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
    inputs: Vec<Input>,
    decal: bool,
    reveal: bool,
    layer_specular: bool,
    base_alpha_response: bool,
    parms: [f32;4],
}

impl Recipe {
    fn sample(&self, uv0: [f32;2], uv1: [f32;2], uv2: [f32;2], green: f32, blue: f32, transform: [f32;4]) -> Result<[[f32;4];4]> {
        let inputs = &self.inputs;
        let (c0,c1,n0,n1) = (inputs[0].sample(uv0),inputs[2].sample(uv1),inputs[1].sample(uv0),inputs[3].sample(uv1));
        let weight = if self.reveal {
            let a = green*f32::from_bits(0x3f7f7cee)+f32::from_bits(0x3a83126f);
            let q = 1.-a;
            let low = (q-self.parms[0]*(a.log2()*self.parms[1]).exp2()).clamp(0.,1.);
            let high = (q+self.parms[0]*(q.log2()*self.parms[1]).exp2()).clamp(0.,1.);
            if high <= low { return Err("Layered reveal thresholds have no interval".into()); }
            ((c1[3]-low)/(high-low)).clamp(0.,1.)
        } else { c1[3]*green };
        let decal = if self.decal { inputs[4].sample(uv2) } else { [1.;4] };
        let mut color = [1.;4];
        for c in 0..3 { color[c]=((c0[c]+weight*(c1[c]-c0[c]))*(1.+blue*(decal[c]-1.))).powi(2); }
        let decode = |value: f32| value*f32::from_bits(0x40808102)+f32::from_bits(0xc0010204);
        let s0 = [decode(n0[0]),decode(n0[1])];
        let s1 = [decode(n1[0]),decode(n1[1])];
        let rotated = [s1[0]*transform[0]+s1[1]*transform[1],s1[0]*transform[2]+s1[1]*transform[3]];
        let slope = [s0[0]+weight*(rotated[0]-s0[0]),s0[1]+weight*(rotated[1]-s0[1])];
        let length = (slope[0]*slope[0]+slope[1]*slope[1]+1.).sqrt();
        let normal = [slope[0]/length,slope[1]/length,1./length,1.];
        let (mut response,mut specular) = ([1.,1.,0.,1.],[0.;4]);
        if self.layer_specular {
            let layer = inputs[4].sample(uv1);
            let k = f32::from_bits(0x3e4ccccd);
            let rgb = std::array::from_fn(|c| (k+weight*(layer[c]-k)).powi(2));
            let gloss = weight*layer[3];
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

fn auxiliary(surface: &Surface, index: usize, decal: bool) -> Result<([f32; 2], [f32; 2], [f32; 4])> {
    let bytes = surface.auxiliary.get(index).ok_or("Layered world vertex has no auxiliary data")?;
    let size = if decal { 12 } else { 8 };
    if bytes.len() != size { return Err(format!("Layered world vertex requires {size} auxiliary bytes, found {}", bytes.len()).into()); }
    let uv = [half::f16::from_bits(u16_at(bytes, 0)).to_f32(), half::f16::from_bits(u16_at(bytes, 2)).to_f32()];
    if !uv.iter().all(|v| v.is_finite()) { return Err("Layered world UV is not finite".into()); }
    let uv2 = if decal { [half::f16::from_bits(u16_at(bytes, 4)).to_f32(), half::f16::from_bits(u16_at(bytes, 6)).to_f32()] } else { uv };
    if !uv2.iter().all(|v| v.is_finite()) { return Err("Layered decal UV is not finite".into()); }
    Ok((uv, uv2, std::array::from_fn(|i| bytes[size - 4 + i] as f32 / 255.)))
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

fn periodic_key(surface: &Surface, first: usize) -> Result<Option<Vec<u32>>> {
    if !constant_frame(surface,first) { return Ok(None); }
    let auxiliary = (0..3).map(|i| auxiliary(surface,first+i,false)).collect::<Result<Vec<_>>>()?;
    if !(1..3).all(|i| surface.colors[first+i][1] == surface.colors[first][1] && auxiliary[i].2 == auxiliary[0].2) { return Ok(None); }
    let offsets: [[f32;2];3] = std::array::from_fn(|i| std::array::from_fn(|axis| auxiliary[i].0[axis]-surface.uv[first+i][axis]));
    if offsets[1] != offsets[0] || offsets[2] != offsets[0] { return Ok(None); }
    let mut key = vec![surface.colors[first][1] as u32];
    key.extend(auxiliary[0].2.map(f32::to_bits));
    key.extend(offsets[0].map(|value| value.rem_euclid(1.).to_bits()));
    Ok(Some(key))
}

fn periodic(native: &Value, recipe: &Recipe, key: &[u32], uri: &str, pack: &mut Pack) -> Result<Value> {
    let width = recipe.inputs.iter().map(|image| image.width).max().ok_or("Periodic material has no inputs")?;
    let height = recipe.inputs.iter().map(|image| image.height).max().unwrap();
    let mut outputs: [Vec<[f32;4]>;4] = std::array::from_fn(|_| Vec::with_capacity(width*height));
    let green = key[0] as f32/255.;
    let transform = std::array::from_fn(|axis| 2.*f32::from_bits(key[axis+1])-1.);
    let offset: [f32;2] = std::array::from_fn(|axis| f32::from_bits(key[axis+5]));
    for y in 0..height { for x in 0..width {
        let uv0 = [(x as f32+0.5)/width as f32,(y as f32+0.5)/height as f32];
        let uv1 = std::array::from_fn(|axis| uv0[axis]+offset[axis]);
        for (output,pixel) in outputs.iter_mut().zip(recipe.sample(uv0,uv1,uv0,green,0.,transform)?) { output.push(pixel); }
    }}
    let material = store(native,uri,width,height,outputs,20,0,pack)?;
    println!("Baked shared periodic recipe {uri}: {width}x{height}, source green {}",key[0]);
    Ok(material)
}

fn mip_chain(width: usize, height: usize, pixels: Vec<[f32; 4]>, role: &str) -> Result<texture::Image> {
    let mut image = texture::Image { width, height, depth:1, faces:1, format:1, flags:0, levels:Vec::new() };
    let (mut w, mut h, mut pixels) = (width, height, pixels);
    loop {
        let mut bytes = Vec::with_capacity(w * h * 4);
        for pixel in &pixels {
            let mut value = *pixel;
            if role == "normal" {
                let length = value[..3].iter().map(|v| v*v).sum::<f32>().sqrt();
                if length < 1e-8 { value = [0., 0., 1., 1.]; }
                else { for channel in &mut value[..3] { *channel /= length; } }
                for channel in &mut value[..3] { *channel = *channel * 0.5 + 0.5; }
            } else if role == "color" {
                for channel in &mut value[..3] {
                    *channel = if *channel <= 0.0031308 { *channel * 12.92 } else { 1.055 * channel.powf(1./2.4) - 0.055 };
                }
            }
            if value.iter().any(|v| !v.is_finite() || !(0. ..=1.).contains(v)) { return Err("Baked material pixel exceeds UNORM range".into()); }
            bytes.extend(value.map(|v| (v*255.).round() as u8));
        }
        image.levels.push(bytes);
        if w == 1 && h == 1 { break; }
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::with_capacity(nw * nh);
        for y in 0..nh { for x in 0..nw {
            let values = [pixels[(y*2).min(h-1)*w + (x*2).min(w-1)], pixels[(y*2).min(h-1)*w + (x*2+1).min(w-1)],
                pixels[(y*2+1).min(h-1)*w + (x*2).min(w-1)], pixels[(y*2+1).min(h-1)*w + (x*2+1).min(w-1)]];
            next.push(std::array::from_fn(|c| values.iter().map(|v| v[c]).sum::<f32>() * 0.25));
        }}
        (w, h, pixels) = (nw, nh, next);
    }
    Ok(image)
}

fn recipe(native: &Value, images: &BTreeMap<String, texture::Image>) -> Result<Recipe> {
    let base_alpha_response = native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1";
    let reveal_decal = native["technique_set"] == "lit_sm_r0c0n0_b1c1n1v1_m2c2";
    let decal = reveal_decal || native["technique_set"] == "lit_sm_r0c0n0_b1c1n1_m2c2";
    let layer_specular = native["technique_set"] == "lit_sm_r0c0n0_b1c1n1s1v1";
    let reveal = reveal_decal || layer_specular || native["technique_set"] == "lit_sm_r0c0n0_b1c1n1v1";
    let (vertex, pixel) = if base_alpha_response { ("pimp_shader_layer_lmap_c9b52d6f.hlsl", "pimp_shader_layer_lmap_cd368409.hlsl") }
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
    let reveal_parms = if reveal {
        if !passes[0]["arguments"].as_array().unwrap().iter().any(|arg| arg["type"] == 6 && arg["value"] == 2294217777_u32 && arg["buffer"] == 1 && arg["location"] == 944 && arg["size"] == 16) {
            return Err("Layered reveal parameter shader binding differs from recovered source".into());
        }
        let constant = native["constants"].as_array().unwrap().iter().find(|value| value["hash"] == 2294217777_u32).ok_or("Layered reveal parameters are missing")?;
        let values: [f32; 4] = serde_json::from_value(constant["literal"].clone())?;
        if !values.iter().all(|value| value.is_finite()) || values[0] <= 0. || values[1] <= 0. { return Err("Layered reveal parameters are outside the recovered domain".into()); }
        values
    } else { [0.;4] };
    let expected = [(0,0),(1,2),(2,5),(3,1),(4,7),(5,6),(6,8),(8,10)];
    let routes = passes[0]["vertex_routes"].as_array().ok_or("Layered vertex routes are missing")?;
    if routes.len() != expected.len() || routes.iter().zip(expected).any(|(route, (source, destination))| route["source"] != source || route["destination"] != destination) {
        return Err("Layered material vertex declaration differs from recovered source".into());
    }
    let mut inputs = Vec::new();
    let mut bindings = vec![(2695565377_u32,20,0), (1507003663,20,257), (3054311504,20,514), (2486480606,20,if decal {1027} else {771})];
    if decal { bindings.push((3054311507,10,772)); }
    if layer_specular { bindings.push((3532022562,20,1028)); }
    for (hash, expected_sampler, location) in bindings {
        let binding = native["textures"].as_array().unwrap().iter().find(|binding| binding["hash"] == hash).ok_or("Layered source texture binding is missing")?;
        let sampler = binding["sampler"].as_u64().and_then(|value| u8::try_from(value).ok()).ok_or("Layered sampler is not a byte")?;
        if (sampler != expected_sampler && !(reveal_decal && hash == 3054311507 && sampler == 74))
            || !passes[0]["arguments"].as_array().unwrap().iter().any(|argument| argument["type"] == 2 && argument["value"] == hash && argument["location"] == location) {
            return Err("Layered texture has an unreviewed sampler or shader slot".into());
        }
        let name = binding["image"].as_str().ok_or("Layered source image name is missing")?;
        let image = images.get(&name.to_ascii_lowercase()).ok_or("Layered source image is missing")?;
        println!("Layered input {name}: {}x{}",image.width,image.height);
        inputs.push(Input::read(image,sampler)?);
    }
    Ok(Recipe {inputs,decal,reveal,layer_specular,base_alpha_response,parms:reveal_parms})
}

fn layered(native: &Value, surfaces: &[Surface], recipe: &Recipe, uri: &str, converted: &mut BTreeMap<String,Value>, pack: &mut Pack) -> Result<Vec<Surface>> {
    let (decal,layer_specular,inputs) = (recipe.decal,recipe.layer_specular,&recipe.inputs);
    let mut charts = Vec::<Chart>::new();
    let mut shared_charts = BTreeMap::<Vec<u32>,(usize,[usize;3])>::new();
    let mut uv_area = 0_f64;
    for (surface_index, surface) in surfaces.iter().enumerate() {
        for first in (0..surface.positions.len()).step_by(3) {
            let varying_frame = !constant_frame(surface,first);
            let auxiliary = (0..3).map(|i| auxiliary(surface, first+i, decal)).collect::<Result<Vec<_>>>()?;
            let layer_uv: Vec<_> = auxiliary.iter().map(|value| value.0).collect();
            let decal_uv: Vec<_> = auxiliary.iter().map(|value| value.1).collect();
            let uv_streams = if decal { vec![&surface.uv[first..first+3],layer_uv.as_slice(),decal_uv.as_slice()] }
                else { vec![&surface.uv[first..first+3],layer_uv.as_slice()] };
            let offsets: Vec<[f32;2]> = uv_streams.iter().enumerate().map(|(stream,uv)| std::array::from_fn(|axis| {
                let clamped = inputs.iter().enumerate().any(|(input,image)| {
                    let source = if input < 2 { 0 } else if decal && input == 4 { 2 } else { 1 };
                    source == stream && image.sampler & (32 << axis) != 0
                });
                if clamped { 0. } else { uv.iter().map(|value| value[axis]).fold(f32::INFINITY,f32::min).floor() }
            })).collect();
            let keys: [Vec<u32>;3] = std::array::from_fn(|i| {
                let mut key = Vec::new();
                for (uv,offset) in uv_streams.iter().zip(&offsets) { for axis in 0..2 { key.push((uv[i][axis]-offset[axis]).to_bits()); } }
                key.push(surface.colors[first+i][1] as u32);
                if decal { key.push(surface.colors[first+i][2] as u32); }
                key.extend(auxiliary[i].2.map(f32::to_bits));
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
                    let uv = if input < 2 { &surface.uv[first..first+3] } else if decal && input == 4 { &decal_uv } else { &layer_uv };
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
    let mut coverage = vec![f64::INFINITY;width*height];
    let mut basis_errors = [0_f64;2];
    let mut basis_texels = 0_usize;
    let mut basis_triangles = 0_usize;
    for chart in &page.charts {
        for triangle in &chart.triangles {
        let surface = &surfaces[triangle.surface];
        let frame = if constant_frame(surface,triangle.first) { None } else { Some(NormalFrame::read(surface,triangle.first)?) };
        if frame.is_some() { basis_triangles+=triangle.copies.len(); }
        let auxiliary = (0..3).map(|i| auxiliary(surface,triangle.first+i,decal)).collect::<Result<Vec<_>>>()?;
        let lower: [usize;2] = std::array::from_fn(|axis| triangle.points.iter().map(|point| point[axis].floor().max(0.) as usize).min().unwrap());
        let upper: [usize;2] = std::array::from_fn(|axis| triangle.points.iter().map(|point| point[axis].ceil().max(0.) as usize).max().unwrap()+2*GUTTER);
        let end = [upper[0].min(chart.width+2*GUTTER),upper[1].min(chart.height+2*GUTTER)];
        for y in lower[1]..end[1] { for x in lower[0]..end[0] {
            let point = [x as f64+0.5-GUTTER as f64,y as f64+0.5-GUTTER as f64];
            let (weights,distance) = triangle_weights(triangle.points,point)?;
            if distance > (GUTTER*GUTTER) as f64 { continue; }
            let index = (chart.y+y)*width+chart.x+x;
            if coverage[index] <= distance { continue; }
            coverage[index]=distance;
            let interpolate = |values: [f32; 3]| values.into_iter().zip(weights).map(|(value, weight)| value as f64*weight).sum::<f64>() as f32;
            let uv0 = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| surface.uv[triangle.first+i][axis])));
            let uv1 = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| auxiliary[i].0[axis])));
            let uv2 = std::array::from_fn(|axis| interpolate(std::array::from_fn(|i| auxiliary[i].1[axis])));
            let transform: [f32; 4] = std::array::from_fn(|axis| 2.*interpolate(std::array::from_fn(|i| auxiliary[i].2[axis]))-1.);
            let green = interpolate(std::array::from_fn(|i| surface.colors[triangle.first+i][1] as f32/255.));
            let blue = interpolate(std::array::from_fn(|i| surface.colors[triangle.first+i][2] as f32/255.));
            let mut pixel = recipe.sample(uv0,uv1,uv2,green,blue,transform)?;
            if let Some(frame) = &frame {
                let (normal,errors) = frame.convert(weights,pixel[1])?;
                pixel[1]=normal;
                for axis in 0..2 { basis_errors[axis]=basis_errors[axis].max(errors[axis]); }
                basis_texels+=1;
            }
            if layer_specular || recipe.base_alpha_response {
                response[index] = pixel[2];
            }
            if layer_specular {
                specular[index] = pixel[3];
            }
            albedo[index]=pixel[0];
            normals[index]=pixel[1];
        }}
        }
    }
    drop(coverage);
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
    let base_alpha_response = native["technique_set"] == "lit_sm_r0c0n0x0_b1c1n1";
    let layer_specular = native["technique_set"] == "lit_sm_r0c0n0_b1c1n1s1v1";
    let [albedo,normals,response,specular] = outputs;
    let mut channels = Vec::new();
    let mut outputs = vec![("color",albedo),("normal",normals)];
    if layer_specular || base_alpha_response { outputs.push(("response",response)); }
    if layer_specular { outputs.push(("native-specular",specular)); }
    let mut native_specular = Value::Null;
    for (role, pixels) in outputs {
        let image = mip_chain(width,height,pixels,role)?;
        let path = format!("{uri}/{role}.ktx2");
        let bytes = texture::encode(&image,role == "color",if role == "native-specular" { "reflectance" } else { role })?;
        texture::validate(&bytes)?;
        pack.entries.insert(path.clone(),Arc::new(bytes));
        if role == "native-specular" { native_specular=json!({"uri":path,"specular_rgb":"linear","gloss_channel":"alpha"}); }
        else {
            if role == "response" && base_alpha_response {
                native_specular=json!({"uri":path,"gloss_channel":"alpha","specular_rgb":"constant_0.04",
                    "source":"Direct-lobe Fresnel floor; native environment Fresnel fit differs"});
            }
            channels.push(json!({"uri":path,"role":role,"layer":0,"color_space":if role == "color" {"srgb"} else {"linear"},"sampler":sampler}));
        }
    }
    let mut material = json!({"name":native["name"],"pbrMetallicRoughness":{"baseColorFactor":[1.,1.,1.,1.],"metallicFactor":0.},
        "extras":{"surface":{"version":3,"identity":native["name"],"native":native,"channels":channels,
            "standard":{"roughness":1.,"metallic":if layer_specular {1.} else {0.},"reflectance":if layer_specular || base_alpha_response {0.5} else {0.},"base_color":[1.,1.,1.,1.]},
            "native_specular":native_specular,
            "presentation":{"alpha":"opaque","cull":"back","fog":true,"visible":true},
            "bake":{"width":width,"height":height,"charts":charts,"source_texel_density":true,"layout":if sampler == 20 {"periodic"} else {"atlas"}},
            "approximations":["Static material bake at source UV texel density; finite rasterization, BC compression and baked mip filtering", "Source normal slopes normalized before runtime texture filtering", "Source specular/gloss fitted to shared metallic/GGX PBR inputs where present; response mips average the fitted inputs"]}}});
    if sampler != 20 {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Coarse atlas mip levels can mix unrelated charts beyond the eight-pixel gutter"));
    }
    if base_alpha_response {
        material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Base color alpha retained in response alpha and fitted from native exponent exp2(13*a) to GGX perceptual roughness (2/(exp2(13*a)+2))^(1/4). Dielectric F0=0.04 uses the native sun-lobe floor; this cannot reproduce the native environment Fresnel/mip fit, direct-lobe geometry/amplitude or inverse-SH compensation exactly. View-dependent lighting is not baked into albedo or emission"));
    }
    Ok(material)
}

pub fn supported(family: &str) -> bool {
    matches!(family,"lit_sm_r0c0n0_b1c1n1" | "lit_sm_r0c0n0_b1c1n1_m2c2" | "lit_sm_r0c0n0_b1c1n1s1v1" | "lit_sm_r0c0n0_b1c1n1v1" | "lit_sm_r0c0n0_b1c1n1v1_m2c2" | "lit_sm_r0c0n0x0_b1c1n1")
}

pub fn world(catalog: &Catalog<'_>, native: &BTreeMap<String, Value>, images: &BTreeMap<String, texture::Image>, converted: &mut BTreeMap<String,Value>, pack: &mut Pack) -> Result<BTreeMap<String, Vec<Surface>>> {
    let required: BTreeMap<_, _> = native.iter().filter(|(_, material)| material["technique_set"].as_str().is_some_and(supported)).collect();
    if required.is_empty() { return Ok(BTreeMap::new()); }
    let mut groups = BTreeMap::<String,Vec<Surface>>::new();
    for (_, zone, root) in catalog.definitions(17) {
        for surface in geometry::world(catalog,zone,root)? {
            let key = surface.material.to_ascii_lowercase();
            if required.contains_key(&key) { groups.entry(key).or_default().push(surface.unindex()?); }
        }
    }
    let mut baked = BTreeMap::<String,Vec<Surface>>::new();
    for (key, material) in required {
        let surfaces = groups.remove(key).ok_or_else(|| format!("Layered material {key} has no supported world geometry"))?;
        let source_counts: BTreeMap<_,_> = surfaces.iter().map(|surface| (surface.name.clone(),surface.indices.len())).collect();
        let input_triangles: usize = surfaces.iter().map(|surface| surface.indices.len()/3).sum();
        let material_recipe = recipe(material,images)?;
        let mut periodic_cache = BTreeMap::<Vec<u32>,String>::new();
        let mut remaining = Vec::new();
        let mut periodic_triangles = 0;
        for surface in surfaces {
            if (!material_recipe.reveal && !material_recipe.base_alpha_response) || material_recipe.decal { remaining.push(surface); continue; }
            let mut triangles = BTreeMap::<Vec<u32>,Vec<usize>>::new();
            let mut variable = Vec::new();
            for triangle in 0..surface.indices.len()/3 {
                if let Some(recipe) = periodic_key(&surface,triangle*3)? { triangles.entry(recipe).or_default().push(triangle); }
                else { variable.push(triangle); }
            }
            for (recipe,triangles) in triangles {
                let identity = if let Some(identity) = periodic_cache.get(&recipe) { identity.clone() } else {
                    let identity = format!("{key}#periodic{}",periodic_cache.len());
                    let uri = format!("prepared/materials/{}/periodic",converted.len());
                    let mut value = periodic(material,&material_recipe,&recipe,&uri,pack)?;
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
            for surface in layered(material,&remaining,&material_recipe,&uri,converted,pack)? { baked.entry(surface.name.clone()).or_default().push(surface); }
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
mod tests {
    use super::*;

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
    fn layered_base_alpha_response_survives_complete_layer_coverage() {
        let input = |pixel| Input {width:1,height:1,sampler:20,pixels:vec![pixel]};
        let mut recipe = Recipe {inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.2,0.4,1.]),input([0.5,0.5,1.,1.])],decal:false,reveal:false,
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
        let mut recipe = Recipe {inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.2,0.4,0.5]),input([0.75,0.25,1.,1.]),
            Input {width:2,height:2,sampler:74,pixels:vec![[0.9,0.8,0.7,1.],[0.1,0.2,0.3,1.],
                [0.5,0.25,0.75,0.],[1.,0.5,0.25,0.]]}],
            decal:true,reveal:true,layer_specular:false,base_alpha_response:false,parms:[0.0713300034403801,0.5,0.,0.]};
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
        let recipe = Recipe {inputs:vec![input([0.2,0.4,0.6,0.]),input([0.5,0.5,1.,1.]),
            input([0.8,0.6,0.2,0.5]),input([0.5,0.5,1.,1.])],
            decal:false,reveal:true,layer_specular:false,base_alpha_response:false,parms:[0.0713300034403801,0.5,0.,0.]};
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
