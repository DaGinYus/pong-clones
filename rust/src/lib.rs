// 2024-03-09
// Matthew Wong
// a 'true' clone of the popular game, Pong
// ref: https://www.pong-story.com/LAWN_TENNIS.pdf

use std::convert::TryInto;
use std::iter;
use godot::prelude::*;
use godot::engine::{Polygon2D, IPolygon2D, Area2D, IArea2D};

// pixel conversion information
// the 'resolution' of the video signal was 455x262 clock signals (60Hz VSYNC)
// the HBLANK signal was 81 CLKs long, for an active video time of 374 CLKs
// the VBLANK signal was 16 CLKs long, for an active video time of 246 CLKS
// 1H is close to 0.14us long and 1V is close to 254us
// the total scanning time for the active area would be 52.36us for the width and 62.48ms for the height
// say we want to define the active area to be 640x480 (VGA)
// then 640px / 53.36us = 12 px/us = 1.68 px/1H 
//      480px / 62.48ms = 1.89 px/ms = 1.95 px/1V
// these are hardcoded, maybe consider making these dynamic based on viewport settings
// a singleton containing game constants could be helpful here
const VIEWPORT_HEIGHT: i32 = 480;
const PX_UNIT_WIDTH: f32 = 1.68;
const PX_UNIT_HEIGHT: f32 = 1.95;
const HBLANK: i32 = 81;
const VBLANK: i32 = 16;
const HSHIFT: i32 = 16;
const PADDLE_MOVE_BY: i32 = 16;

struct Pong;

#[gdextension]
unsafe impl ExtensionLibrary for Pong {}

// the original circuitry resulted in the net being shifted to the left instead
// we can add HSHIFT to center everything, or we can turn it off for 'accuracy'
fn hclk_to_xpos(hclk: i32) -> i32 {
    let hclk_since_hblank = hclk - HBLANK + HSHIFT;
    (hclk_since_hblank as f32 * PX_UNIT_WIDTH) as i32
}

fn hclk_to_px(hclk: i32) -> i32 {
    (hclk as f32 * PX_UNIT_WIDTH) as i32
}

fn vclk_to_ypos(vclk: i32) -> i32 {
    let vclk_since_vblank = vclk - VBLANK;
    (vclk_since_vblank as f32 * PX_UNIT_HEIGHT) as i32
}

fn vclk_to_px(vclk: i32) -> i32 {
    (vclk as f32 * PX_UNIT_HEIGHT) as i32
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    net: Gd<Net>,
    paddle_l: Gd<Paddle>,
    paddle_r: Gd<Paddle>,
    scoreboard: Gd<ScoreDisplay>,
    base: Base<Node>
}

#[godot_api]
impl INode for Main {
    fn init(base: Base<Node>) -> Self {
        Self {
            net: Net::new_alloc(),
            paddle_l: Paddle::from_side(PlayerSide::Left),
            paddle_r: Paddle::from_side(PlayerSide::Right),
            scoreboard: ScoreDisplay::new_alloc(),
            base
        } 
    }

    fn ready(&mut self) {
        let net_clone = self.net.clone();
        let paddle_l_clone = self.paddle_l.clone();
        let paddle_r_clone = self.paddle_r.clone();
        let scoreboard_clone = self.scoreboard.clone();
        self.base_mut().add_child(net_clone.clone().upcast());
        self.base_mut().add_child(paddle_l_clone.upcast());
        self.base_mut().add_child(paddle_r_clone.upcast());
        self.base_mut().add_child(scoreboard_clone.upcast());
    }
}

#[derive(Clone)]
struct Rect<T> {
    x: T,
    y: T,
    w: T,
    h: T,
}

impl Into<Rect<f32>> for Rect<i32> {
    fn into(self) -> Rect<f32> {
        Rect {
            x: self.x as f32,
            y: self.y as f32,
            w: self.w as f32,
            h: self.h as f32,
        }
    }
}

impl<T> Rect<T> {
    fn new(x: T, y: T, w: T, h: T) -> Self {
        Self { x, y, w, h }
    }

    fn from_clk(hclk: i32, vclk: i32, w: i32, h: i32) -> Rect::<i32> {
        Rect::<i32> {
            x: hclk_to_xpos(hclk),
            y: vclk_to_ypos(vclk),
            w: hclk_to_px(w),
            h: vclk_to_px(h),
        }
    }
}

fn polygon2d_add_rect(polygon: &mut Gd<Polygon2D>, rect: Rect<i32>, redraw: bool) {
    let mut vertices = match redraw {
        true => PackedVector2Array::new(),
        false => polygon.get_polygon(),
    };
    let rect_f: Rect<f32> = rect.clone().into();
    vertices.push(Vector2::new(rect_f.x, rect_f.y));
    vertices.push(Vector2::new(rect_f.x+rect_f.w, rect_f.y));
    vertices.push(Vector2::new(rect_f.x+rect_f.w, rect_f.y+rect_f.h));
    vertices.push(Vector2::new(rect_f.x, rect_f.y+rect_f.h));
    polygon.set_polygon(vertices);
}

fn polygon2d_set_indices(polygon: &mut Gd<Polygon2D>) {
    let mut polygon_indices = Array::<Variant>::new();
    let num_vertices = polygon.get_polygon().len();
    for i in 0..num_vertices/4 {
        let base = 4*i as i32;
        polygon_indices.push(PackedInt32Array::from(&[base, base+1, base+2, base+3]).to_variant());
    }
    polygon.set_polygons(polygon_indices);
}

#[derive(GodotClass)]
#[class(init, base=Polygon2D)]
struct Net {
    base: Base<Polygon2D>
}

#[godot_api]
impl IPolygon2D for Net {
    fn ready(&mut self) {
        self.draw();
    }
}

impl Net {
    // the net is triggered at 256H from the HRST signal
    // the net is dependent on a 4V signal for the segments, and is only one pulse wide
    // this means the net should be drawn with roughly 2x8 segments 8px apart
    fn draw(&mut self) {
        let net_left_edge = hclk_to_xpos(256);
        let net_segment_spacing: usize = vclk_to_px(8).try_into().unwrap();

        let net_width = hclk_to_px(1);
        let net_height = vclk_to_px(4);
        for i in (0..VIEWPORT_HEIGHT).step_by(net_segment_spacing) {
            let i_int = i as i32;
            let rect = Rect::new(net_left_edge, i_int, net_width, net_height);
            polygon2d_add_rect(&mut self.base_mut(), rect, false);
        }
        polygon2d_set_indices(&mut self.base_mut());
    }
}

enum PlayerSide {
    Left,
    Right
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Paddle {
    ypos: i32,
    side: PlayerSide,
    polygon: Gd<Polygon2D>,
    base: Base<Area2D>
}

#[godot_api]
impl IArea2D for Paddle {
    fn init(base: Base<Area2D>) -> Self {
        let init_y = vclk_to_ypos(128);
        Self {
            ypos: init_y,
            side: PlayerSide::Left,
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
            PlayerSide::Left => {
                if input.is_action_pressed("up_l".into()) { self.move_up() }
                if input.is_action_pressed("dn_l".into()) { self.move_down()}
            },
            PlayerSide::Right => {
                if input.is_action_pressed("up_r".into()) { self.move_up() }
                if input.is_action_pressed("dn_r".into()) { self.move_down() }
            }
        }
        self.draw();
    }
}

impl Paddle {
    fn from_side(side: PlayerSide) -> Gd<Self> {
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
        let xpos = match self.side {
            PlayerSide::Left => hclk_to_xpos(128),
            PlayerSide::Right => hclk_to_xpos(128+256),
        };
        let ypos = self.ypos;
        let bat_height = vclk_to_px(15);
        let bat_width = hclk_to_px(4);
        let rect = Rect::new(xpos, ypos, bat_width, bat_height);
        polygon2d_add_rect(&mut self.polygon, rect, true);
    }

    // the paddles actually could not move the entire range
    // based on watching old pong footage, it looks like the maximum range tops
    // out at the top line of the score counter, or 32V
    fn move_up(&mut self) {
        let min_ypos = vclk_to_ypos(32);
        let new_ypos = self.ypos - PADDLE_MOVE_BY;
        if new_ypos >= min_ypos {
            self.ypos = new_ypos
        } else {
            self.ypos = min_ypos
        }
    }

    // i assume the maximum would also be around 16V from the bottom of the screen
    fn move_down(&mut self) {
        let bat_height = vclk_to_px(15);
        let max_ypos = VIEWPORT_HEIGHT - vclk_to_px(16) - bat_height;
        let new_ypos = self.ypos + PADDLE_MOVE_BY;
        if new_ypos <= max_ypos {
            self.ypos = new_ypos
        } else {
            self.ypos = max_ypos
        }
    }
}

#[derive(GodotClass)]
#[class(base=Node2D)]
struct ScoreDisplay {
    score: [i32; 2],
    polygon: Gd<Polygon2D>,
    base: Base<Node2D>
}

#[godot_api]
impl INode2D for ScoreDisplay {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            score: [0, 0],
            polygon: Polygon2D::new_alloc(),
            base
        }
    }

    fn ready(&mut self) {
        let polygon_clone = self.polygon.clone();
        self.base_mut().add_child(polygon_clone.upcast());
    }

    fn process(&mut self, _delta: f64) {
        self.draw_seven_segment();
    }
}

impl ScoreDisplay {
    //    _a_
    // f |_g_| b
    // e |___| c
    //     d
    // converts to an array of values [a, b, c, d, e, f, g]
    fn n_to_seven_segment(n: i32) -> Option<[i8; 7]> {
        match n {
            0 => Some([1, 1, 1, 1, 1, 1, 0]),
            1 => Some([0, 1, 1, 0, 0, 0, 0]),
            2 => Some([1, 1, 0, 1, 1, 0, 1]),
            3 => Some([1, 1, 1, 1, 0, 0, 1]),
            4 => Some([0, 1, 1, 0, 0, 1, 1]),
            5 => Some([1, 0, 1, 1, 0, 1, 1]),
            6 => Some([1, 0, 1, 1, 1, 1, 1]),
            7 => Some([1, 1, 1, 0, 0, 0, 0]),
            8 => Some([1, 1, 1, 1, 1, 1, 1]),
            9 => Some([1, 1, 1, 0, 0, 1, 1]),
            _ => None
        }
    }

    // the score windows were positioned 32V from the top of the screen
    // for two digit scores, the numbers were 4H apart from each other
    // the leftmost edge was at 144H, so the next leftmost would be at 160H
    // for P2 on the right, the leftmost segment was at 336H and the second digit was at 352H
    // name the horizontal segments 'rows' and the vertical segments 'cols'
    fn draw_seven_segment(&mut self) {
        self.polygon.set_polygon(PackedVector2Array::new());
        let offset_vclk = 32;
        for (player, score) in self.score.iter().enumerate() {
            let ones_digit = score % 10;
            let tens_digit = score / 10;
            // trick to calculate offsets using the indices of the scores
            let ones_hclk = 175 + (player as i32)*192;
            // make a list of rects, then zip/map with the n_to_seven_segment and draw only if 1
            if tens_digit != 0 {
                let tens_seg = ScoreDisplay::n_to_seven_segment(tens_digit).unwrap();
                let tens_hclk = ones_hclk - 16;
                let tens_seg_rects = [
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk, 16, 4),
                    Rect::<i32>::from_clk(tens_hclk+12, offset_vclk, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk+12, offset_vclk+16, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+29, 16, 4),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+16, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+12, 16, 4),
                ];
                for (seg_is_on, seg_rect) in iter::zip(tens_seg, tens_seg_rects) {
                    if seg_is_on == 1 { polygon2d_add_rect(&mut self.polygon, seg_rect, false) }
                }
            }
            let ones_seg = ScoreDisplay::n_to_seven_segment(ones_digit).unwrap();
            let ones_seg_rects = [
                Rect::<i32>::from_clk(ones_hclk, offset_vclk, 16, 4),
                Rect::<i32>::from_clk(ones_hclk+12, offset_vclk, 4, 16),
                Rect::<i32>::from_clk(ones_hclk+12, offset_vclk+16, 4, 16),
                Rect::<i32>::from_clk(ones_hclk, offset_vclk+29, 16, 4),
                Rect::<i32>::from_clk(ones_hclk, offset_vclk+16, 4, 16),
                Rect::<i32>::from_clk(ones_hclk, offset_vclk, 4, 16),
                Rect::<i32>::from_clk(ones_hclk, offset_vclk+12, 16, 4),
            ];
            for (seg_is_on, seg_rect) in iter::zip(ones_seg, ones_seg_rects) {
                if seg_is_on == 1 { polygon2d_add_rect(&mut self.polygon, seg_rect, false) }
            }
        }
        polygon2d_set_indices(&mut self.polygon);
    }
}