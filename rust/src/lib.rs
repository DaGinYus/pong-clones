// 2024-03-09
// Matthew Wong
// a 'true' clone of the popular game, Pong
// ref: https://www.pong-story.com/LAWN_TENNIS.pdf

// pixel conversion information
// the 'resolution' of the video signal was 455x262 clock signals (60Hz VSYNC)
// the HBLANK signal was 81 CLKs long, for an active video time of 374 CLKs
// the VBLANK signal was 16 CLKs long, for an active video time of 246 CLKS
// 1H is close to 0.14us long and 1V is close to 254us
// the total scanning time for the active area would be 52.36us for the width and 62.48ms for the height
// say we want to define the active area to be 640x480 (VGA)
// then 640px / 53.36us = 12 px/us = 1.68 px/1H 
//      480px / 62.48ms = 1.89 px/ms = 1.95 px/1V

use godot::prelude::*;
use godot::engine::{Polygon2D, IPolygon2D};

const VIEWPORT_W: i32 = 640;
const VIEWPORT_H: i32 = 480;
const PX_UNIT_W: f32 = 1.68;
const PX_UNIT_H: f32 = 1.95;

struct Pong;

#[gdextension]
unsafe impl ExtensionLibrary for Pong {}

fn hclk_to_x(px: i32) -> i32{
    (px as f32 * PX_UNIT_W) as i32
}

fn vclk_to_y(px: i32) -> i32{
    (px as f32 * PX_UNIT_H) as i32
}

#[derive(GodotClass)]
#[class(init, base=Node)]
struct Main {
    base: Base<Node>
}

#[godot_api]
impl INode for Main {
    fn ready(&mut self) {
        let mut net = Net::new_alloc();
        self.base_mut().add_child(net.clone().upcast());
        net.bind_mut().draw();
    }
}

#[derive(GodotClass)]
#[class(init, base=Polygon2D)]
struct Net {
    base: Base<Polygon2D>
}

impl Net {
    // the net is triggered at 256H from the HRST signal, or 175H from the beginning of draw time
    // the net is dependent on a 4V signal for the segments, and is only one pulse wide
    // this means the net should be drawn with roughly 2x8 segments 8px apart
    fn draw(&mut self) {
        let net_start = hclk_to_x(175);
        let mut polygons = PackedVector2Array::new();
        let mut polygon_indices = Array::<Variant>::new();
        let net_segment_spacing: usize = hclk_to_x(8).try_into().unwrap();

        let net_to_f = net_start as f32;
        let net_width_to_f = PX_UNIT_W as f32;
        let net_height_to_f = 4.0*PX_UNIT_H as f32;
        for i in (0..VIEWPORT_H).step_by(net_segment_spacing) {
            let to_float_i = i as f32;
            polygons.push(Vector2::new(net_to_f, to_float_i));
            polygons.push(Vector2::new(net_to_f+net_width_to_f, to_float_i));
            polygons.push(Vector2::new(net_to_f+net_width_to_f, to_float_i+net_height_to_f));
            polygons.push(Vector2::new(net_to_f, to_float_i+net_height_to_f));
        }
        let num_vertices = polygons.len();
        for i in 0..num_vertices/4 {
            let base = 4*i as i32;
            polygon_indices.push(PackedInt32Array::from(&[base, base+1, base+2, base+3]).to_variant());
        }
        self.base_mut().set_polygon(polygons);
        self.base_mut().set_polygons(polygon_indices);
    }
}

enum PaddleSide {
    Left,
    Right,
}

#[derive(GodotClass)]
#[class(base=Polygon2D)]
struct Paddle {
    xpos: i32,
    ypos: i32,
    side: PaddleSide,
    base: Base<Polygon2D>
}

#[godot_api]
impl IPolygon2D for Paddle {
    fn init(base: Base<Polygon2D>) -> Self {
        Self {
            xpos: 0,
            ypos: 0,
            side: PaddleSide::Left,
            base,
        }
    }
}

impl Paddle {
    // the paddle was triggered at 128H and was 4H wide
    // it was composed of 15 'segments,' each one taking up one line
    // the ball's vertical velocity is determined by which segment it hits
    fn draw(&mut self) {
        let polygon = PackedVector2Array::new();
        match self.side {
            PaddleSide::Left => (),
            PaddleSide::Right => (),
        }
    }
}