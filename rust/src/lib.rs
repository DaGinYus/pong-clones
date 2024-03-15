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
use godot::engine::{Polygon2D, Area2D, IArea2D, Viewport, InputEvent, InputEventKey};

// these are hardcoded, maybe consider making these dynamic based on viewport settings
const VIEWPORT_WIDTH: i32 = 640;
const VIEWPORT_HEIGHT: i32 = 480;
const PX_UNIT_WIDTH: f32 = 1.68;
const PX_UNIT_HEIGHT: f32 = 1.95;
const HBLANK: i32 = 81;
const VBLANK: i32 = 16;
const HSYNC_DLY: i32 = 16;
const VSYNC_DLY: i32 = 8;

struct Pong;

#[gdextension]
unsafe impl ExtensionLibrary for Pong {}

fn hclk_to_xpos(hclk: i32) -> i32 {
    let hclk_since_hsync = hclk - HBLANK + HSYNC_DLY;
    (hclk_since_hsync as f32 * PX_UNIT_WIDTH) as i32
}

fn hclk_to_interval(hclk: i32) -> i32 {
    (hclk as f32 * PX_UNIT_WIDTH) as i32
}

fn vclk_to_ypos(vclk: i32) -> i32 {
    let vclk_since_vsync = vclk - VBLANK + VSYNC_DLY;
    (vclk_since_vsync as f32 * PX_UNIT_HEIGHT) as i32
}

fn vclk_to_interval(vclk: i32) -> i32 {
    (vclk as f32 * PX_UNIT_HEIGHT) as i32
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    net: Gd<Net>,
    paddle_l: Gd<Paddle>,
    paddle_r: Gd<Paddle>,
    base: Base<Node>
}

#[godot_api]
impl INode for Main {
    fn init(base: Base<Node>) -> Self {
        Self {
            net: Net::new_alloc(),
            paddle_l: Paddle::from_side(PaddleSide::Left),
            paddle_r: Paddle::from_side(PaddleSide::Right),
            base
        } 
    }

    fn ready(&mut self) {
        let net_clone = self.net.clone();
        let paddle_l_clone = self.paddle_l.clone();
        let paddle_r_clone = self.paddle_r.clone();
        self.base_mut().add_child(net_clone.clone().upcast());
        self.net.bind_mut().draw();
        self.base_mut().add_child(paddle_l_clone.upcast());
        self.base_mut().add_child(paddle_r_clone.upcast());
    }
}

#[derive(GodotClass)]
#[class(init, base=Polygon2D)]
struct Net {
    base: Base<Polygon2D>
}

impl Net {
    // the net is triggered at 256H from the HRST signal
    // the net is dependent on a 4V signal for the segments, and is only one pulse wide
    // this means the net should be drawn with roughly 2x8 segments 8px apart
    fn draw(&mut self) {
        let net_left_edge = hclk_to_xpos(256) as f32;
        let mut polygons = PackedVector2Array::new();
        let mut polygon_indices = Array::<Variant>::new();
        let net_segment_spacing: usize = vclk_to_interval(8).try_into().unwrap();

        let net_width = hclk_to_interval(1) as f32;
        let net_height = vclk_to_interval(4) as f32;
        for i in (0..VIEWPORT_HEIGHT).step_by(net_segment_spacing) {
            let to_float_i = i as f32;
            polygons.push(Vector2::new(net_left_edge, to_float_i));
            polygons.push(Vector2::new(net_left_edge+net_width, to_float_i));
            polygons.push(Vector2::new(net_left_edge+net_width, to_float_i+net_height));
            polygons.push(Vector2::new(net_left_edge, to_float_i+net_height));
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
    Right
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Paddle {
    ypos: i32,
    side: PaddleSide,
    polygon: Gd<Polygon2D>,
    base: Base<Area2D>
}

#[godot_api]
impl IArea2D for Paddle {
    fn init(base: Base<Area2D>) -> Self {
        let init_y = vclk_to_ypos(128);
        Self {
            ypos: init_y,
            side: PaddleSide::Left,
            polygon: Polygon2D::new_alloc(),
            base
        }
    }

    fn ready(&mut self) {
        let polygon_clone = self.polygon.clone();
        self.base_mut().add_child(polygon_clone.upcast());
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();
        match self.side {
            PaddleSide::Left => {
                if input.is_action_pressed("up_l".into()) { self.ypos += 1 }
                if input.is_action_pressed("dn_l".into()) { self.ypos -= 1 }
            },
            PaddleSide::Right => {
                if input.is_action_pressed("up_r".into()) { self.ypos += 1 }
                if input.is_action_pressed("dn_r".into()) { self.ypos -= 1 }
            }
        }
        self.draw();
    }
}

impl Paddle {
    fn from_side(side: PaddleSide) -> Gd<Self> {
        let init_y = vclk_to_ypos(128);
        Gd::from_init_fn(|base| {
            Self {
                ypos: init_y,
                side,
                polygon: Polygon2D::new_alloc(),
                base
            }
        })
    }

    // the paddle was triggered at when the 128H clock signal went high and was 4H wide
    // it was composed of 15 'segments,' each composed of one HSYNC, or one line
    // the ball's vertical velocity is determined by which segment it hits
    // the new vertices are always pushed, this might be slow--consider only updating when y changes
    fn draw(&mut self) {
        let mut vertices = PackedVector2Array::new();
        let xpos = match self.side {
            PaddleSide::Left => hclk_to_xpos(128),
            PaddleSide::Right => hclk_to_xpos(128+256),
        } as f32;
        let ypos = self.ypos as f32;
        let bat_height = vclk_to_interval(15) as f32;
        let bat_width = hclk_to_interval(4) as f32;
        vertices.push(Vector2::new(xpos, ypos));
        vertices.push(Vector2::new(xpos+bat_width, ypos));
        vertices.push(Vector2::new(xpos+bat_width, ypos+bat_height));
        vertices.push(Vector2::new(xpos, ypos+bat_height));
        self.polygon.set_polygon(vertices);
    }
}