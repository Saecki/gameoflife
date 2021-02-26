use std::time::Duration;

use noise::{NoiseFn, Perlin, Seedable};
use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::{keyboard::Keycode, rect::Rect};

const WIDTH: usize = 160;
const HEIGHT: usize = 90;
const DEFAULT_FRAME_DELAY: u32 = 30;

struct Board {
    fields: [[bool; WIDTH]; HEIGHT],
}

impl Board {
    fn new(fields: [[bool; WIDTH]; HEIGHT]) -> Self {
        Board { fields }
    }

    fn generate(method: impl Fn(usize, usize) -> bool) -> Self {
        let mut fields = [[false; WIDTH]; HEIGHT];

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                fields[y][x] = method(x, y);
            }
        }

        Board::new(fields)
    }

    fn perlin() -> Self {
        let noise = Perlin::new().set_seed(rand::random());

        Self::generate(|x, y| {
            let val = noise.get([x as f64 / 42.0, y as f64 / 42.0]);

            val < -0.2
        })
    }

    fn random() -> Self {
        Self::generate(|_, _| rand::random::<f64>() < 0.3)
    }

    fn next(&self) -> Self {
        let mut next = [[false; WIDTH]; HEIGHT];

        for (y, row) in self.fields.iter().enumerate() {
            for (x, field) in row.iter().enumerate() {
                let neighbours = self.neighbours(x, y);

                next[y][x] = match (field, neighbours) {
                    (true, n) if n == 2 || n == 3 => true,
                    (false, 3) => true,
                    (_, _) => false,
                }
            }
        }

        Board::new(next)
    }

    fn neighbours(&self, x: usize, y: usize) -> usize {
        let x_low = if x > 0 { x - 1 } else { 0 };
        let x_high = if x == WIDTH - 1 { WIDTH - 1 } else { x + 1 };
        let y_low = if y > 0 { y - 1 } else { 0 };
        let y_high = if y == HEIGHT - 1 { HEIGHT - 1 } else { y + 1 };

        let mut neighbours = 0;
        for y in y_low..=y_high {
            for x in x_low..=x_high {
                if self.fields[y][x] == true {
                    neighbours += 1;
                }
            }
        }

        neighbours
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("fun", 1600, 900)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    canvas.set_draw_color(Color::RGB(20, 20, 20));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut frame_delay = DEFAULT_FRAME_DELAY;

    let mut board = Board::random();

    'running: loop {
        for e in event_pump.poll_iter() {
            match e {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => match code {
                    Keycode::Q => break 'running,
                    Keycode::R => board = Board::random(),
                    Keycode::P => board = Board::perlin(),
                    Keycode::Num0 => frame_delay = DEFAULT_FRAME_DELAY,
                    Keycode::Equals => frame_delay += 1,
                    Keycode::Minus => {
                        if frame_delay > 0 {
                            frame_delay -= 1
                        }
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        board = board.next();

        let (width, height) = canvas.output_size().unwrap();
        let tile_width = width / WIDTH as u32;
        let tile_height = height / HEIGHT as u32;

        canvas.set_draw_color(Color::RGB(20, 20, 20));
        canvas.clear();

        for (y, row) in board.fields.iter().enumerate() {
            for (x, &field) in row.iter().enumerate() {
                if field {
                    canvas.set_draw_color(Color::RGB(200, 200, 200));
                } else {
                    canvas.set_draw_color(Color::RGB(20, 20, 20));
                }
                canvas
                    .fill_rect(Rect::new(
                        tile_width as i32 * x as i32,
                        tile_height as i32 * y as i32,
                        tile_width,
                        tile_height,
                    ))
                    .ok();
            }
        }

        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / frame_delay));
    }
}
