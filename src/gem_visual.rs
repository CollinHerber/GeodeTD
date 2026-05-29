use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::gem::{GEM_KINDS, GRADE_LADDER, GemGrade, GemKind};

const GEM_IMAGE_SIZE: u32 = 96;

#[derive(Resource)]
pub struct GemImages {
    handles: Vec<Handle<Image>>,
    empty: Handle<Image>,
}

impl GemImages {
    pub fn new(images: &mut Assets<Image>) -> Self {
        let mut handles = Vec::with_capacity(GEM_KINDS.len() * GRADE_LADDER.len());

        for gem in GEM_KINDS {
            for grade in GRADE_LADDER {
                handles.push(images.add(gem_image(gem, grade)));
            }
        }

        let empty = images.add(Image::new(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![255, 255, 255, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));

        Self { handles, empty }
    }

    pub fn handle(&self, gem: GemKind, grade: GemGrade) -> Handle<Image> {
        self.handles[gem_index(gem) * GRADE_LADDER.len() + grade.tier()].clone()
    }

    pub fn empty(&self) -> Handle<Image> {
        self.empty.clone()
    }
}

impl FromWorld for GemImages {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        Self::new(&mut images)
    }
}

fn gem_index(gem: GemKind) -> usize {
    match gem {
        GemKind::Ruby => 0,
        GemKind::Sapphire => 1,
        GemKind::Topaz => 2,
        GemKind::Emerald => 3,
        GemKind::Amethyst => 4,
        GemKind::Diamond => 5,
    }
}

fn gem_image(gem: GemKind, grade: GemGrade) -> Image {
    let mut data = vec![0; (GEM_IMAGE_SIZE * GEM_IMAGE_SIZE * 4) as usize];
    let polygon = silhouette(grade);
    let base = gem.srgb();
    let tier = grade.tier() as f32;
    let facet_width = 0.025 + tier * 0.004;
    let glow_radius = match grade {
        GemGrade::Cut => 0.08,
        GemGrade::Perfect => 0.16,
        _ => 0.0,
    };

    for y in 0..GEM_IMAGE_SIZE {
        for x in 0..GEM_IMAGE_SIZE {
            let point = pixel_to_point(x, y);
            let edge_distance = distance_to_edges(point, &polygon);
            let inside = point_in_polygon(point, &polygon);
            let mut alpha = if inside {
                (edge_distance / 0.035).clamp(0.0, 1.0)
            } else if glow_radius > 0.0 && edge_distance < glow_radius {
                (1.0 - edge_distance / glow_radius)
                    * if grade == GemGrade::Perfect {
                        0.34
                    } else {
                        0.18
                    }
            } else {
                0.0
            };

            if alpha == 0.0 {
                continue;
            }

            let mut rgb = base;
            let vertical_light = (point.y + 1.0) * 0.5;
            let side_light = ((-point.x + point.y) * 0.25 + 0.5).clamp(0.0, 1.0);
            let grade_light = 0.88 + tier * 0.075;
            let mut brightness = (0.55 + vertical_light * 0.32 + side_light * 0.18) * grade_light;

            if point.y > 0.18 && point.x < 0.05 {
                brightness += 0.18 + tier * 0.015;
            }
            if point.y < -0.42 || point.x > 0.48 {
                brightness -= 0.15;
            }
            if edge_distance < 0.055 && inside {
                brightness *= 0.66;
            }

            let highlight = facet_highlight(point, grade, facet_width);
            if highlight > 0.0 {
                rgb = mix_rgb(rgb, [1.0, 1.0, 1.0], highlight * (0.34 + tier * 0.04));
                brightness += highlight * 0.22;
            }

            let shadow = facet_shadow(point, grade, facet_width);
            if shadow > 0.0 {
                brightness *= 1.0 - shadow * 0.42;
            }

            if !inside {
                rgb = mix_rgb(rgb, [1.0, 1.0, 1.0], 0.25);
                brightness = 0.95;
            }

            alpha = alpha.clamp(0.0, 1.0);
            write_pixel(&mut data, x, y, scale_rgb(rgb, brightness), alpha);
        }
    }

    Image::new(
        Extent3d {
            width: GEM_IMAGE_SIZE,
            height: GEM_IMAGE_SIZE,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn silhouette(grade: GemGrade) -> Vec<Vec2> {
    match grade {
        GemGrade::Chipped => vec![
            Vec2::new(-0.58, 0.42),
            Vec2::new(-0.05, 0.76),
            Vec2::new(0.56, 0.34),
            Vec2::new(0.48, -0.46),
            Vec2::new(-0.18, -0.76),
            Vec2::new(-0.68, -0.12),
        ],
        GemGrade::Flawed => vec![
            Vec2::new(-0.62, 0.14),
            Vec2::new(-0.34, 0.68),
            Vec2::new(0.34, 0.68),
            Vec2::new(0.64, 0.14),
            Vec2::new(0.08, -0.80),
            Vec2::new(-0.42, -0.56),
        ],
        GemGrade::Regular => vec![
            Vec2::new(-0.66, 0.12),
            Vec2::new(-0.36, 0.70),
            Vec2::new(0.36, 0.70),
            Vec2::new(0.66, 0.12),
            Vec2::new(0.34, -0.64),
            Vec2::new(-0.34, -0.64),
        ],
        GemGrade::Cut => vec![
            Vec2::new(-0.70, 0.10),
            Vec2::new(-0.42, 0.70),
            Vec2::new(0.42, 0.70),
            Vec2::new(0.70, 0.10),
            Vec2::new(0.22, -0.78),
            Vec2::new(-0.22, -0.78),
        ],
        GemGrade::Perfect => vec![
            Vec2::new(-0.76, 0.08),
            Vec2::new(-0.48, 0.72),
            Vec2::new(0.0, 0.84),
            Vec2::new(0.48, 0.72),
            Vec2::new(0.76, 0.08),
            Vec2::new(0.26, -0.82),
            Vec2::new(0.0, -0.92),
            Vec2::new(-0.26, -0.82),
        ],
    }
}

fn facet_highlight(point: Vec2, grade: GemGrade, width: f32) -> f32 {
    let lines: &[(Vec2, Vec2)] = match grade {
        GemGrade::Chipped => &[
            (Vec2::new(-0.48, 0.34), Vec2::new(0.42, 0.22)),
            (Vec2::new(-0.04, 0.70), Vec2::new(-0.18, -0.62)),
        ],
        GemGrade::Flawed => &[
            (Vec2::new(-0.34, 0.62), Vec2::new(0.08, -0.72)),
            (Vec2::new(0.34, 0.62), Vec2::new(-0.42, -0.48)),
        ],
        GemGrade::Regular => &[
            (Vec2::new(0.0, 0.68), Vec2::new(0.0, -0.62)),
            (Vec2::new(-0.56, 0.10), Vec2::new(0.56, 0.10)),
            (Vec2::new(-0.34, 0.62), Vec2::new(0.34, -0.58)),
        ],
        GemGrade::Cut => &[
            (Vec2::new(0.0, 0.68), Vec2::new(0.0, -0.76)),
            (Vec2::new(-0.60, 0.08), Vec2::new(0.60, 0.08)),
            (Vec2::new(-0.38, 0.66), Vec2::new(0.22, -0.74)),
            (Vec2::new(0.38, 0.66), Vec2::new(-0.22, -0.74)),
        ],
        GemGrade::Perfect => &[
            (Vec2::new(0.0, 0.80), Vec2::new(0.0, -0.88)),
            (Vec2::new(-0.66, 0.08), Vec2::new(0.66, 0.08)),
            (Vec2::new(-0.44, 0.70), Vec2::new(0.26, -0.80)),
            (Vec2::new(0.44, 0.70), Vec2::new(-0.26, -0.80)),
            (Vec2::new(-0.18, 0.78), Vec2::new(0.58, -0.02)),
            (Vec2::new(0.18, 0.78), Vec2::new(-0.58, -0.02)),
        ],
    };

    lines
        .iter()
        .map(|(start, end)| line_intensity(point, *start, *end, width))
        .fold(0.0, f32::max)
}

fn facet_shadow(point: Vec2, grade: GemGrade, width: f32) -> f32 {
    let lines: &[(Vec2, Vec2)] = match grade {
        GemGrade::Chipped => &[(Vec2::new(-0.52, -0.08), Vec2::new(0.40, -0.44))],
        GemGrade::Flawed => &[
            (Vec2::new(0.16, 0.34), Vec2::new(0.28, -0.42)),
            (Vec2::new(-0.22, 0.28), Vec2::new(-0.05, -0.08)),
        ],
        GemGrade::Regular => &[(Vec2::new(-0.50, -0.18), Vec2::new(0.42, -0.34))],
        GemGrade::Cut | GemGrade::Perfect => &[
            (Vec2::new(-0.62, -0.18), Vec2::new(0.52, -0.36)),
            (Vec2::new(0.46, 0.28), Vec2::new(0.20, -0.76)),
        ],
    };

    lines
        .iter()
        .map(|(start, end)| line_intensity(point, *start, *end, width * 0.9))
        .fold(0.0, f32::max)
}

fn line_intensity(point: Vec2, start: Vec2, end: Vec2, width: f32) -> f32 {
    let distance = distance_to_segment(point, start, end);
    if distance >= width {
        0.0
    } else {
        1.0 - distance / width
    }
}

fn pixel_to_point(x: u32, y: u32) -> Vec2 {
    Vec2::new(
        ((x as f32 + 0.5) / GEM_IMAGE_SIZE as f32) * 2.0 - 1.0,
        1.0 - ((y as f32 + 0.5) / GEM_IMAGE_SIZE as f32) * 2.0,
    )
}

fn point_in_polygon(point: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];

    for current in polygon {
        let crosses = (current.y > point.y) != (previous.y > point.y);
        if crosses {
            let x_intersection = (previous.x - current.x) * (point.y - current.y)
                / (previous.y - current.y)
                + current.x;
            if point.x < x_intersection {
                inside = !inside;
            }
        }
        previous = *current;
    }

    inside
}

fn distance_to_edges(point: Vec2, polygon: &[Vec2]) -> f32 {
    polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .map(|(start, end)| distance_to_segment(point, *start, *end))
        .fold(f32::MAX, f32::min)
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let segment = end - start;
    let t = ((point - start).dot(segment) / segment.length_squared()).clamp(0.0, 1.0);
    point.distance(start + segment * t)
}

fn mix_rgb(a: [f32; 3], b: [f32; 3], amount: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * amount,
        a[1] + (b[1] - a[1]) * amount,
        a[2] + (b[2] - a[2]) * amount,
    ]
}

fn scale_rgb(rgb: [f32; 3], brightness: f32) -> [f32; 3] {
    [
        (rgb[0] * brightness).clamp(0.0, 1.0),
        (rgb[1] * brightness).clamp(0.0, 1.0),
        (rgb[2] * brightness).clamp(0.0, 1.0),
    ]
}

fn write_pixel(data: &mut [u8], x: u32, y: u32, rgb: [f32; 3], alpha: f32) {
    let index = ((y * GEM_IMAGE_SIZE + x) * 4) as usize;
    data[index] = (rgb[0] * 255.0).round() as u8;
    data[index + 1] = (rgb[1] * 255.0).round() as u8;
    data[index + 2] = (rgb[2] * 255.0).round() as u8;
    data[index + 3] = (alpha * 255.0).round() as u8;
}
