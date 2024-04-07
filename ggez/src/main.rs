use ggez::{Context, ContextBuilder};
use ggez::graphics;
use ggez::event;
use ggez::error;

struct State {}

impl event::EventHandler<error::GameError> for State {
  fn update(&mut self, ctx: &mut Context) -> ggez::GameResult {
      Ok(())
  }
  fn draw(&mut self, ctx: &mut Context) -> ggez::GameResult {
      Ok(())
  }
}

fn main() {
    let state = State {};

    let (ctx, event_loop) = ContextBuilder::new("pong-ggez", "")
        .build()
        .expect("Failed to create context");
}