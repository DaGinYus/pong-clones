use ggez::*;
use ggez::graphics::Color;

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
const VIEWPORT_WIDTH: f32 = 640.0;
const VIEWPORT_HEIGHT: f32 = 480.0;
const PX_UNIT_WIDTH: f32 = 1.68;
const PX_UNIT_HEIGHT: f32 = 1.95;
const HBLANK: i32 = 81;
const VBLANK: i32 = 16;
const HSHIFT: i32 = 16;
const PADDLE_MOVE_BY: f32 = 1.0;
const WIN_SCORE: i32 = 11;

// utility funcs for converting pong timing values to pixels
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

struct Net {}
impl Net {
    fn draw(&mut self, ctx: &mut Context) {
        let net_width = hclk_to_px(1) as u32;
        let seg_height = vclk_to_px(4) as u32;
        let seg_spacing: usize = vclk_to_px(8).try_into().unwrap();
        let image = graphics::Image::from_color(&ctx.gfx, net_width, seg_height, Some(Color::WHITE));
        let mut segments = graphics::InstanceArray::new(&ctx.gfx, image);
        let mut canvas = graphics::Canvas::from_frame(&ctx.gfx, None);
        for i in (0..VIEWPORT_HEIGHT as i32).step_by(seg_spacing) {
            let ypos = i as f32 - VIEWPORT_HEIGHT/2.0;
            let loc = glam::vec2(0.0, ypos);
            segments.push(graphics::DrawParam::default().dest(loc));
            println!("{:?}", segments);
        }
        canvas.draw(&segments, graphics::DrawParam::default());
    }
}

struct Paddle {

}

struct State {
    net: Net,
    paddles: [Paddle; 2],
}

impl State {
    fn new() -> Self {
        Self {
            net: Net {},
            paddles: [Paddle {}, Paddle {}],
        }
    }
}

impl event::EventHandler<error::GameError> for State {
  fn update(&mut self, ctx: &mut Context) -> ggez::GameResult {
    self.net.draw(ctx);
    Ok(())
  }
  fn draw(&mut self, ctx: &mut Context) -> ggez::GameResult {
    Ok(())
  }
}

fn main() {
    let state = State::new();
    let window_mode = conf::WindowMode::default().dimensions(VIEWPORT_WIDTH, VIEWPORT_HEIGHT);
    let backend = conf::Backend::OnlyPrimary;
    let (ctx, event_loop) = ContextBuilder::new("pong-ggez", "")
        .window_mode(window_mode)
        .backend(backend)
        .build()
        .expect("Failed to create ggez context");
    event::run(ctx, event_loop, state)
}