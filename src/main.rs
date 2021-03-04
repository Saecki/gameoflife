use std::ops::{Index, IndexMut};
use std::time::{Duration, SystemTime};

use noise::{NoiseFn, Seedable};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;

const DEFAULT_VIRT_FPS: u32 = 30;

const FPS: u32 = 60;
const WIDTH: usize = 160;
const HEIGHT: usize = 90;

const PERLIN_SCALE: f64 = 0.16;
const BILLOW_SCALE: f64 = 0.08;
const WORLEY_SCALE: f64 = 0.16;

struct Board<const W: usize, const H: usize> {
    fields: Vec<bool>,
}

impl<const W: usize, const H: usize> Index<usize> for Board<W, H> {
    type Output = [bool];

    fn index(&self, index: usize) -> &Self::Output {
        &self.fields[index * W..(index + 1) * W]
    }
}

impl<const W: usize, const H: usize> IndexMut<usize> for Board<W, H> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.fields[index * W..(index + 1) * W]
    }
}

impl<const W: usize, const H: usize> Board<W, H> {
    fn new(fields: Vec<bool>) -> Self {
        Self { fields }
    }

    fn clear() -> Self {
        Self::new(vec![false; H * W])
    }

    fn generate(method: impl Fn(usize, usize) -> bool + Sync + Send) -> Self {
        let size = H * W;
        let fields: Vec<bool> = (0..size)
            .into_iter()
            .map(move |n| {
                let x = n % W;
                let y = n / W;

                method(x, y)
            })
            .collect();

        Self::new(fields)
    }

    fn random() -> Self {
        Self::generate(|_, _| rand::random())
    }

    fn perlin() -> Self {
        let noise = noise::Perlin::new().set_seed(rand::random());

        Self::generate(|x, y| {
            let val = noise.get([x as f64 * PERLIN_SCALE, y as f64 * PERLIN_SCALE]);

            val < 0.0
        })
    }

    fn billow() -> Self {
        let noise = noise::Billow::new().set_seed(rand::random());

        Self::generate(|x, y| {
            let val = noise.get([x as f64 * BILLOW_SCALE, y as f64 * BILLOW_SCALE]);

            val < -0.5
        })
    }

    fn worley() -> Self {
        let noise = noise::Worley::new().set_seed(rand::random());

        Self::generate(|x, y| {
            let val = noise.get([x as f64 * WORLEY_SCALE, y as f64 * WORLEY_SCALE]);

            val < -0.5
        })
    }

    fn glider() -> Self {
        let mut new = Self::clear();

        #[rustfmt::skip]
        new.draw(10, 10, &[
            "  #",
            "# #",
            " ##"
        ]);

        new
    }

    fn glider_gun() -> Self {
        let mut new = Self::clear();

        #[rustfmt::skip]
        new.draw(10, 10, &[
            "                        #           ",
            "                      # #           ",
            "            ##      ##            ##",
            "           #   #    ##            ##",
            "##        #     #   ##              ",
            "##        #   # ##    # #           ",
            "          #     #       #           ",
            "           #   #                    ",
            "            ##                      ",
        ]);

        new
    }

    fn rows(&self) -> impl Iterator<Item = &[bool]> {
        (0..H).into_iter().map(move |y| &self[y])
    }

    fn next(&self) -> Self {
        Self::generate(|x, y| {
            let val = self[y][x];
            let neighbours = self.neighbours(x, y);

            match (val, neighbours) {
                (true, 2) => true,
                (_, 3) => true,
                (_, _) => false,
            }
        })
    }

    fn neighbours(&self, x: usize, y: usize) -> usize {
        let x_low = if x == 0 { 0 } else { x - 1 };
        let x_high = if x == W - 1 { W - 1 } else { x + 1 };
        let y_low = if y == 0 { 0 } else { y - 1 };
        let y_high = if y == H - 1 { H - 1 } else { y + 1 };

        let mut neighbours = 0;
        for _y in y_low..=y_high {
            for _x in x_low..=x_high {
                if (_x, _y) == (x, y) {
                    continue;
                }
                if self[_y][_x] {
                    neighbours += 1;
                }
            }
        }

        neighbours as usize
    }

    fn draw(&mut self, x_offset: usize, y_offset: usize, grid: &[&str]) {
        for (y, row) in grid.iter().enumerate() {
            for (x, val) in row.chars().enumerate() {
                self[y_offset + y][x_offset + x] = val == '#';
            }
        }
    }

    fn line(&mut self, x1: usize, y1: usize, x2: usize, y2: usize, val: bool) {
        let xd = x2 as isize - x1 as isize;
        let yd = y2 as isize - y1 as isize;

        let md = xd.abs().max(yd.abs());
        let xf = xd as f32 / md as f32;
        let yf = yd as f32 / md as f32;

        for i in 0..=md {
            let x = (x1 as f32 + (xf * i as f32).round()) as usize;
            let y = (y1 as f32 + (yf * i as f32).round()) as usize;
            self[y][x] = val;
        }
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

    let mut update_rate = DEFAULT_VIRT_FPS;
    let mut last_update = std::time::SystemTime::now();
    let mut last_render = std::time::SystemTime::now();
    let mut pause = false;
    let mut last_x = 0;
    let mut last_y = 0;

    let mut board = Board::<WIDTH, HEIGHT>::random();

    'running: loop {
        let (width, height) = canvas.output_size().unwrap();
        let tile_width = width / WIDTH as u32;
        let tile_height = height / HEIGHT as u32;
        let current_time = SystemTime::now();

        for e in event_pump.poll_iter() {
            match e {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(code),
                    ..
                } => match code {
                    Keycode::Q => break 'running,
                    Keycode::C => board = Board::clear(),
                    Keycode::R => board = Board::random(),
                    Keycode::P => board = Board::perlin(),
                    Keycode::B => board = Board::billow(),
                    Keycode::W => board = Board::worley(),
                    Keycode::L => board = Board::glider(),
                    Keycode::G => board = Board::glider_gun(),
                    Keycode::Space => pause = !pause,
                    Keycode::Num0 => update_rate = DEFAULT_VIRT_FPS,
                    Keycode::Equals => update_rate += 1,
                    Keycode::Minus => {
                        if update_rate > 1 {
                            update_rate -= 1
                        }
                    }
                    _ => (),
                },
                Event::MouseButtonDown {
                    x, y, mouse_btn, ..
                } => {
                    last_x = x as usize / tile_width as usize;
                    last_y = y as usize / tile_height as usize;
                    board[last_y][last_x] = matches!(mouse_btn, MouseButton::Left);
                }
                Event::MouseMotion {
                    x, y, mousestate, ..
                } => {
                    let vx = x as usize / tile_width as usize;
                    let vy = y as usize / tile_height as usize;

                    if mousestate.left() {
                        board.line(last_x, last_y, vx, vy, true);
                    } else if mousestate.right() {
                        board.line(last_x, last_y, vx, vy, false);
                    }

                    last_x = vx;
                    last_y = vy;
                }
                _ => (),
            }
        }

        if !pause {
            let measured_nanos = current_time.duration_since(last_update).unwrap().as_nanos();
            let nanos = 1_000_000_000 / update_rate as u128;
            if measured_nanos > nanos {
                board = board.next();
                last_update = current_time;
            }
        }

        let measured_nanos = current_time.duration_since(last_render).unwrap().as_nanos();
        let nanos = 1_000_000_000 / FPS as u128;
        if measured_nanos > nanos {
            canvas.set_draw_color(Color::RGB(20, 20, 20));
            canvas.clear();

            for (y, row) in board.rows().enumerate() {
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
            last_render = current_time;
        }

        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 1000));
    }
}
