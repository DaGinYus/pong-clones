// 2024-03-09
// Matthew Wong
// a 'true' clone of the popular game, Pong
// ref: https://www.pong-story.com/LAWN_TENNIS.pdf

use std::convert::TryInto;
use std::iter;
use godot::prelude::*;
use godot::engine::{Polygon2D, CollisionPolygon2D, CollisionShape2D, RectangleShape2D, IPolygon2D, Area2D, IArea2D};

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
const VIEWPORT_WIDTH: i32 = 640;
const VIEWPORT_HEIGHT: i32 = 480;
const PX_UNIT_WIDTH: f32 = 1.68;
const PX_UNIT_HEIGHT: f32 = 1.95;
const HBLANK: i32 = 81;
const VBLANK: i32 = 16;
const HSHIFT: i32 = 16;
const PADDLE_MOVE_BY: f32 = 1.0;
const WIN_SCORE: i32 = 11;

struct Pong;

#[gdextension]
unsafe impl ExtensionLibrary for Pong {}

// the original circuitry resulted in the net being shifted to the left instead
// we can add HSHIFT to center everything, or we can turn it off for 'accuracy'
fn hclk_to_xpos(hclk: i32) -> f32 {
    let hclk_since_hblank = hclk - HBLANK + HSHIFT;
    hclk_since_hblank as f32 * PX_UNIT_WIDTH
}

fn hclk_to_px(hclk: i32) -> i32 {
    (hclk as f32 * PX_UNIT_WIDTH) as i32
}

fn vclk_to_ypos(vclk: i32) -> f32 {
    let vclk_since_vblank = vclk - VBLANK;
    vclk_since_vblank as f32 * PX_UNIT_HEIGHT
}

fn vclk_to_px(vclk: i32) -> i32 {
    (vclk as f32 * PX_UNIT_HEIGHT) as i32
}

#[derive(GodotClass)]
#[class(base=Node)]
struct Main {
    paddle_l: Gd<Paddle>,
    paddle_r: Gd<Paddle>,
    ball: Gd<Ball>,
    wall_l: Gd<Wall>,
    wall_r: Gd<Wall>,
    attract_mode: bool,
    base: Base<Node>
}

#[godot_api]
impl INode for Main {
    fn init(base: Base<Node>) -> Self {
        Self {
            paddle_l: Paddle::from_side(PlayerSide::Left),
            paddle_r: Paddle::from_side(PlayerSide::Right),
            ball: Ball::new_alloc(),
            wall_l: Wall::new_alloc(),
            wall_r: Wall::new_alloc(),
            attract_mode: false,
            base
        } 
    }

    fn process(&mut self, _delta: f64) {
        let input = Input::singleton();
        if input.is_action_pressed("enter".into()) {
            if self.attract_mode {
                self.attract_mode = false;
                self.new_game();
            }
        }
    }

    fn ready(&mut self) {
        self.new_game();
    }
}

#[godot_api]
impl Main {
    fn clear_children(&mut self) {
        for mut child in self.base_mut().get_children().iter_shared().skip(1) {
            child.queue_free();
        }
    }

    fn new_game(&mut self) {
        self.clear_children();
        self.paddle_l = Paddle::from_side(PlayerSide::Left);
        self.paddle_r = Paddle::from_side(PlayerSide::Right);
        self.ball = Ball::new_alloc();
        self.wall_l = Wall::new_alloc();
        self.wall_r = Wall::new_alloc();
        self.base_mut().add_child(Net::new_alloc().upcast());
        let paddle_l = self.paddle_l.clone();
        let paddle_r = self.paddle_r.clone();
        self.base_mut().add_child(paddle_l.upcast());
        self.base_mut().add_child(paddle_r.upcast());
        let ball = self.ball.clone();
        let ball_callable = self.ball.callable("on_score_updated");
        self.base_mut().add_child(ball.upcast());
        let wall_l = self.wall_l.clone();
        self.wall_l.bind_mut().set_side(PlayerSide::Left);
        let wall_r = self.wall_r.clone();
        self.wall_r.bind_mut().set_side(PlayerSide::Right);
        self.base_mut().add_child(wall_l.upcast());
        self.base_mut().add_child(wall_r.upcast());
        self.base_mut().add_child(VBounds::new_alloc().upcast());
        let mut display = ScoreDisplay::new_alloc();
        let display_callable = display.callable("on_score");
        self.base_mut().add_child(display.clone().upcast());

        self.wall_l.connect("scored".into(), display_callable.clone());
        self.wall_r.connect("scored".into(), display_callable.clone());
        display.connect("score_updated".into(), ball_callable.clone());
        display.connect("game_over".into(), self.base().callable("attract_mode"));
    }

    #[func]
    fn attract_mode(&mut self) {
        self.attract_mode = true;
        self.paddle_l.queue_free();
        self.paddle_r.queue_free();
        self.wall_l.bind_mut().attract_mode = true;
        self.wall_r.bind_mut().attract_mode = true;
        self.ball.bind_mut().serve();
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
            x: hclk_to_xpos(hclk) as i32,
            y: vclk_to_ypos(vclk) as i32,
            w: hclk_to_px(w),
            h: vclk_to_px(h),
        }
    }
}

fn set_vertices_from_rect(vertices: &mut PackedVector2Array, rect: &Rect<i32>) {
    let rect_f: Rect<f32> = rect.clone().into();
    vertices.push(Vector2::new(rect_f.x, rect_f.y));
    vertices.push(Vector2::new(rect_f.x+rect_f.w, rect_f.y));
    vertices.push(Vector2::new(rect_f.x+rect_f.w, rect_f.y+rect_f.h));
    vertices.push(Vector2::new(rect_f.x, rect_f.y+rect_f.h));
}

trait AddRect {
    fn add_rect(&mut self, rect: &Rect<i32>);
}

impl AddRect for Polygon2D {
    fn add_rect(&mut self, rect: &Rect<i32>) {
        let mut vertices = self.get_polygon();
        set_vertices_from_rect(&mut vertices, rect);
        self.set_polygon(vertices);
    }
}

impl AddRect for CollisionPolygon2D {
    fn add_rect(&mut self, rect: &Rect<i32>) {
        let mut vertices = self.get_polygon();
        set_vertices_from_rect(&mut vertices, rect);
        self.set_polygon(vertices);
    }
}

fn polygon_set_indices(polygon: &mut Gd<Polygon2D>) {
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
        let net_left_edge = hclk_to_xpos(256) as i32;
        let net_segment_spacing: usize = vclk_to_px(8).try_into().unwrap();

        let net_width = hclk_to_px(1);
        let net_height = vclk_to_px(4);
        for i in (0..VIEWPORT_HEIGHT).step_by(net_segment_spacing) {
            let i_int = i as i32;
            let rect = Rect::new(net_left_edge, i_int, net_width, net_height);
            self.base_mut().add_rect(&rect);
        }
        polygon_set_indices(&mut self.base_mut());
    }
}

#[derive(Clone)]
enum PlayerSide {
    Left,
    Right
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Paddle {
    ypos: f32,
    side: PlayerSide,
    polygon: Gd<Polygon2D>,
    collision_segments: [Gd<CollisionShape2D>; 7],
    base: Base<Area2D>
}

#[godot_api]
impl IArea2D for Paddle {
    fn init(base: Base<Area2D>) -> Self {
        let init_y = vclk_to_ypos(120);
        let segments: [Gd<CollisionShape2D>; 7] = std::array::from_fn(|_| CollisionShape2D::new_alloc());
        Self {
            ypos: init_y,
            side: PlayerSide::Left,
            polygon: Polygon2D::new_alloc(),
            collision_segments: segments,
            base
        }
    }

    fn ready(&mut self) {
        let polygon = self.polygon.clone();
        self.base_mut().add_child(polygon.upcast());
        let collision_segments = self.collision_segments.clone();
        for collision in collision_segments {
            self.base_mut().add_child(collision.clone().upcast());
        }
        self.draw();
        self.set_collision_segments();
        let callable = self.base().callable("on_paddle_area_shape_entered");
        self.base_mut().connect("area_shape_entered".into(), callable);
    }

    fn process(&mut self, delta: f64) {
        let input = Input::singleton();
        let xpos = match self.side {
            PlayerSide::Left => hclk_to_xpos(128),
            PlayerSide::Right => hclk_to_xpos(128+256),
        };
        match self.side {
            PlayerSide::Left => {
                if input.is_action_pressed("up_l".into()) { self.move_up(delta) }
                if input.is_action_pressed("dn_l".into()) { self.move_down(delta)}
            },
            PlayerSide::Right => {
                if input.is_action_pressed("up_r".into()) { self.move_up(delta) }
                if input.is_action_pressed("dn_r".into()) { self.move_down(delta) }
            }
        }
        let pos = Vector2::new(xpos as f32, self.ypos as f32);
        self.base_mut().set_global_position(pos);
    }
}

#[godot_api]
impl Paddle {
    fn from_side(side: PlayerSide) -> Gd<Self> {
        let init_y = vclk_to_ypos(120);
        let collision_segments: [Gd<CollisionShape2D>; 7] = std::array::from_fn(|_| CollisionShape2D::new_alloc());
        Gd::from_init_fn(|base| {
            Self {
                ypos: init_y,
                side,
                polygon: Polygon2D::new_alloc(),
                collision_segments: collision_segments,
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
        let pos = Vector2::new(xpos as f32, self.ypos as f32);
        self.base_mut().set_global_position(pos);
        let bat_height = vclk_to_px(16);
        let bat_width = hclk_to_px(4);
        let rect = Rect::new(0, 0, bat_width, bat_height);
        self.polygon.add_rect(&rect);
    }

    fn set_collision_segments(&mut self) {
        let bat_width = hclk_to_px(4);
        let collision_offsets_vclk = [0, 2, 4, 6, 10, 12, 14];
        let segment_heights = [2, 2, 2, 4, 2, 2, 2];
        for (i, segment) in self.collision_segments.iter_mut().enumerate() {
            let segment_height = segment_heights[i] as f32 * PX_UNIT_HEIGHT;
            let offset = collision_offsets_vclk[i] as f32 * PX_UNIT_HEIGHT;
            let mut collision_shape = RectangleShape2D::new_gd();
            collision_shape.set_size(Vector2::new(bat_width as f32, segment_height));
            segment.set_position(Vector2::new(0.0, offset));
            segment.set_shape(collision_shape.upcast());
        }
    }

    // the paddles actually could not move the entire range
    // based on watching old pong footage, it looks like the maximum range tops
    // out at the top line of the score counter, or 32V
    fn move_up(&mut self, delta: f64) {
        let min_ypos = vclk_to_ypos(32);
        let new_ypos = self.ypos - PADDLE_MOVE_BY * VIEWPORT_HEIGHT as f32 * delta as f32;
        if new_ypos >= min_ypos {
            self.ypos = new_ypos
        } else {
            self.ypos = min_ypos
        }
    }

    // i assume the maximum would also be around 16V from the bottom of the screen
    fn move_down(&mut self, delta: f64) {
        let bat_height = vclk_to_px(16);
        let max_ypos = (VIEWPORT_HEIGHT - vclk_to_px(16) - bat_height) as f32;
        let new_ypos = self.ypos + PADDLE_MOVE_BY * VIEWPORT_HEIGHT as f32 * delta as f32;
        if new_ypos <= max_ypos {
            self.ypos = new_ypos
        } else {
            self.ypos = max_ypos
        }
    }

    #[func]
    fn on_paddle_area_shape_entered(_area_rid: Variant, area: Gd<Area2D>, _area_shape_index: i32, local_shape_index: i32) {
        if let Ok(mut area) = area.try_cast::<Ball>() {
            if !area.bind().has_collided {
                area.bind_mut().has_collided = true;
                let yvel = match local_shape_index {
                    0 => -3,
                    1 => -2,
                    2 => -1,
                    3 => 0,
                    4 => 1,
                    5 => 2,
                    6 => 3,
                    _ => 0,
                };
                area.bind_mut().yvel = yvel;
                area.bind_mut().xvel *= -1;
                area.bind_mut().hit_counter += 1;
            }
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
        let polygon = self.polygon.clone();
        self.base_mut().add_child(polygon.upcast());
    }

    fn process(&mut self, _delta: f64) {
        self.draw_seven_segment();
    }
}

#[godot_api]
impl ScoreDisplay {
    #[signal]
    fn score_updated();

    #[signal]
    fn game_over();

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
                let tens_hclk = ones_hclk - 32;
                let tens_seg_rects = [
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk, 16, 4),
                    Rect::<i32>::from_clk(tens_hclk+12, offset_vclk, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk+12, offset_vclk+16, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+29, 16, 4),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+16, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk, 4, 16),
                    Rect::<i32>::from_clk(tens_hclk, offset_vclk+13, 16, 4),
                ];
                for (seg_is_on, seg_rect) in iter::zip(tens_seg, tens_seg_rects) {
                    if seg_is_on == 1 { self.polygon.add_rect(&seg_rect) }
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
                Rect::<i32>::from_clk(ones_hclk, offset_vclk+13, 16, 4),
            ];
            for (seg_is_on, seg_rect) in iter::zip(ones_seg, ones_seg_rects) {
                if seg_is_on == 1 { self.polygon.add_rect(&seg_rect) }
            }
        }
        polygon_set_indices(&mut self.polygon);
    }

    #[func]
    fn on_score(&mut self, side: GString) {
        let side = side.to_string();
        if side == "left".to_string() {
            self.score[0] += 1;
            if self.score[0] == WIN_SCORE {
                self.base_mut().emit_signal("game_over".into(), &[]);
                return
            }
            self.base_mut().emit_signal("score_updated".into(), &[]);
        } else if side == "right".to_string() {
            self.score[1] += 1;
            if self.score[1] == WIN_SCORE {
                self.base_mut().emit_signal("game_over".into(), &[]);
                return
            }
            self.base_mut().emit_signal("score_updated".into(), &[]);
        }
    }
}

// velocity discretization in the original was as follows:
// vertical had possible values ranging from 7 to 13
// the value 10 corresponded to strictly horizontal movement
// this is a table of screen heights/second
// val | ht/s
// 13  | -0.695
// 12  | -0.462
// 11  | -0.226
// 10  |  0
//  9  |  0.228
//  8  |  0.455
//  7  |  0.680
//
// horizontal speed increases corresponding to the number of hits
// hits | wd/s
// <4   | 0.26
// 4-11 | 0.39
// 12+  | 0.53

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Ball {
    pos: Vector2,
    xvel: i32,
    yvel: i32,
    spawn: Vector2,
    polygon: Gd<Polygon2D>,
    collision: Gd<CollisionShape2D>,
    has_collided: bool,
    hit_counter: i32,
    base: Base<Area2D>
}

#[godot_api]
impl IArea2D for Ball {
    fn init(base: Base<Area2D>) -> Self {
        let spawn_x = hclk_to_xpos(256);
        let spawn_y = vclk_to_ypos(128);
        Self {
            pos: Vector2::new(spawn_x, spawn_y),
            xvel: 0,
            yvel: 10,
            spawn: Vector2::new(spawn_x, spawn_y),
            polygon: Polygon2D::new_alloc(),
            collision: CollisionShape2D::new_alloc(),
            has_collided: false,
            hit_counter: 0,
            base
        }
    }

    fn ready(&mut self) {
        let polygon = self.polygon.clone();
        let collision = self.collision.clone();
        self.base_mut().add_child(polygon.upcast());
        self.base_mut().add_child(collision.upcast());
        self.draw();
        self.serve();
    }

    fn process(&mut self, delta: f64) {
        let xvel_positive = if self.xvel > 0 { true } else { false };
        self.xvel = match self.hit_counter {
            x if x < 4 => if xvel_positive { 1 } else { -1 },
            x if x < 12 => if xvel_positive { 2 } else { -2 },
            x if x >= 12 => if xvel_positive { 3 } else { -3 },
            _ => 0,
        };
        let height_sec = match self.yvel {
            -3 => -0.695,
            -2 => -0.462,
            -1 => -0.226,
            0 => 0.0,
            1 => 0.228,
            2 => 0.455,
            3 => 0.680,
            _ => 0.0,
        };
        let width_sec = match self.xvel {
            -3 => -0.53,
            -2 => -0.39,
            -1 => -0.26,
            0 => 0.0,
            1 => 0.26,
            2 => 0.39,
            3 => 0.53,
            _ => 0.0,
        };
        // renable collision when ball is clear of the net (to fix issues with segment collision)
        let area_clear_range = hclk_to_xpos(144)..hclk_to_xpos(368);
        if self.has_collided == true && area_clear_range.contains(&self.pos.x) {
            self.has_collided = false;
        }
        let y_px_sec = height_sec * VIEWPORT_HEIGHT as f32;
        let x_px_sec = width_sec * VIEWPORT_WIDTH as f32;
        let xpos = x_px_sec * delta as f32;
        let ypos = y_px_sec * delta as f32;
        self.pos += Vector2::new(xpos, ypos);
        let pos = self.pos;
        self.base_mut().set_global_position(pos);
    }
}

#[godot_api]
impl Ball {
    fn draw(&mut self) {
        let spawn = self.spawn;
        self.base_mut().set_global_position(spawn);
        let ball_height = vclk_to_px(4);
        let ball_width = hclk_to_px(4);
        let rect = Rect::new(0, 0, ball_width, ball_height);
        self.polygon.add_rect(&rect);
        let mut collision_shape = RectangleShape2D::new_gd();
        collision_shape.set_size(Vector2::new(ball_width as f32, 1.0));
        self.collision.set_shape(collision_shape.upcast());
    }

    #[func]
    fn serve(&mut self) {
        self.hit_counter = 0;
        let spawn = self.spawn;
        self.pos = spawn;
        self.base_mut().set_global_position(spawn);
    }

    #[func]
    fn on_score_updated(&mut self) {
        let mut timer = self.base().get_tree().unwrap().create_timer(1.5).unwrap();
        timer.connect("timeout".into(), self.base().callable("serve"));
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct Wall {
    collision: Gd<CollisionPolygon2D>,
    side: PlayerSide,
    attract_mode: bool,
    base: Base<Area2D>
}

#[godot_api]
impl IArea2D for Wall {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            collision: CollisionPolygon2D::new_alloc(),
            side: PlayerSide::Left,
            attract_mode: false,
            base
        }
    }

    fn ready(&mut self) {
        let collision = self.collision.clone();
        self.base_mut().add_child(collision.upcast());
        let callable = self.base().callable("on_wall_area_entered");
        self.base_mut().connect("area_entered".into(), callable);
    }
}

#[godot_api]
impl Wall {
    #[signal]
    fn scored(side: GString);

    fn set_side(&mut self, side: PlayerSide) {
        match side {
            PlayerSide::Left => {
                let position = Rect::new(-11, 0, 10, VIEWPORT_HEIGHT);
                self.collision.add_rect(&position);
            }
            PlayerSide::Right => {
                let position = Rect::new(VIEWPORT_WIDTH+1, 0, 10, VIEWPORT_HEIGHT);
                self.collision.add_rect(&position);
            }
        }
        self.side = side;
    }

    #[func]
    fn on_wall_area_entered(&mut self, area: Gd<Area2D>) {
        if let Ok(mut area) = area.try_cast::<Ball>() {
            if !self.attract_mode {
                match self.side {
                PlayerSide::Left => self.base_mut().emit_signal("scored".into(), &[Variant::from("right")]),
                PlayerSide::Right => self.base_mut().emit_signal("scored".into(), &[Variant::from("left")]),
                };
            } else {
                area.bind_mut().xvel *= -1;
            }
        }
    }
}

#[derive(GodotClass)]
#[class(base=Area2D)]
struct VBounds {
    ceiling: Gd<CollisionPolygon2D>,
    floor: Gd<CollisionPolygon2D>,
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for VBounds {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            ceiling: CollisionPolygon2D::new_alloc(),
            floor: CollisionPolygon2D::new_alloc(),
            base
        }
    }

    fn ready(&mut self) {
        let ceiling_rect = Rect::new(0, -10, VIEWPORT_WIDTH, 10);
        let floor_rect = Rect::new(0, VIEWPORT_HEIGHT, VIEWPORT_WIDTH, 10);
        self.ceiling.add_rect(&ceiling_rect);
        self.floor.add_rect(&floor_rect);
        let ceiling = self.ceiling.clone();
        let floor = self.floor.clone();
        let mut base_ref = self.base_mut().clone();
        let callable = self.base().callable("on_vbounds_area_shape_entered");
        base_ref.add_child(ceiling.upcast());
        base_ref.add_child(floor.upcast());
        base_ref.connect("area_shape_entered".into(), callable);
    }
}

#[godot_api]
impl VBounds {
    #[func]
    fn on_vbounds_area_shape_entered(_area_rid: Variant, area: Gd<Area2D>, _area_shape_index: i32, local_shape_index: i32) {
        if let Ok(mut area) = area.try_cast::<Ball>() {
            let yvel = area.bind().yvel;
            // 0 index is ceiling
            // this approach should guard against clipping
            if local_shape_index == 0 && yvel < 0 {
                area.bind_mut().yvel *= -1;
            } else if local_shape_index == 1 && yvel > 0 {
                area.bind_mut().yvel *= -1;
            }
        }
    }
}