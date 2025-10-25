use cubecl::prelude::*;
use cubecl::server::Handle;
use minifb::{Key, Window, WindowOptions};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// ============================================================================
// CONSTANTS - Hardware Specification
// ============================================================================

const SCREEN_WIDTH: usize = 800;
const SCREEN_HEIGHT: usize = 600;
const CELL_SIZE: usize = 20;
const GRID_WIDTH: usize = SCREEN_WIDTH / CELL_SIZE;
const GRID_HEIGHT: usize = SCREEN_HEIGHT / CELL_SIZE;
const TICK_DURATION: Duration = Duration::from_millis(120);
const TARGET_FPS: u64 = 60;

// ============================================================================
// TYPES - Basic Building Blocks
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: u32,
    y: u32,
}

impl Position {
    fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }

    fn move_by(&self, direction: Direction) -> Self {
        let (dx, dy) = direction.delta();
        let new_x = (self.x as i32 + dx).rem_euclid(GRID_WIDTH as i32) as u32;
        let new_y = (self.y as i32 + dy).rem_euclid(GRID_HEIGHT as i32) as u32;
        Self::new(new_x, new_y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    fn delta(&self) -> (i32, i32) {
        match self {
            Direction::Up => (0, -1),
            Direction::Down => (0, 1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }

    fn opposite(&self) -> Direction {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Color {
    r: f32,
    g: f32,
    b: f32,
}

impl Color {
    const GREEN_BG: Color = Color { r: 0.0, g: 0.4, b: 0.0 };
    const RED_APPLE: Color = Color { r: 0.9, g: 0.0, b: 0.0 };
    const BLACK_SNAKE: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

    fn to_pixel(&self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        (r << 16) | (g << 8) | b
    }
}

// ============================================================================
// GAME STATE - Core Logic
// ============================================================================

struct Snake {
    body: VecDeque<Position>,
    direction: Direction,
    next_direction: Direction,
}

impl Snake {
    fn new(head: Position) -> Self {
        let mut body = VecDeque::new();
        body.push_back(head);
        body.push_back(Position::new(head.x - 1, head.y));
        body.push_back(Position::new(head.x - 2, head.y));
        
        Self {
            body,
            direction: Direction::Right,
            next_direction: Direction::Right,
        }
    }

    fn head(&self) -> Position {
        *self.body.front().unwrap()
    }

    fn set_direction(&mut self, dir: Direction) {
        if dir != self.direction.opposite() {
            self.next_direction = dir;
        }
    }

    fn advance(&mut self) -> Position {
        self.direction = self.next_direction;
        let new_head = self.head().move_by(self.direction);
        self.body.push_front(new_head);
        new_head
    }

    fn shrink(&mut self) {
        self.body.pop_back();
    }

    fn contains(&self, pos: Position) -> bool {
        self.body.iter().any(|&p| p == pos)
    }

    fn serialize(&self) -> Vec<u32> {
        let mut data = Vec::with_capacity(self.body.len() * 2);
        for pos in &self.body {
            data.push(pos.x);
            data.push(pos.y);
        }
        data
    }
}

struct GameState {
    snake: Snake,
    apple: Position,
    score: u32,
    game_over: bool,
    last_tick: Instant,
}

impl GameState {
    fn new() -> Self {
        Self {
            snake: Snake::new(Position::new(
                GRID_WIDTH as u32 / 2,
                GRID_HEIGHT as u32 / 2,
            )),
            apple: Position::new(10, 10),
            score: 0,
            game_over: false,
            last_tick: Instant::now(),
        }
    }

    fn handle_input(&mut self, input: Input) {
        if let Some(dir) = input.direction {
            self.snake.set_direction(dir);
        }
    }

    fn tick(&mut self) {
        if self.game_over {
            return;
        }

        if self.last_tick.elapsed() < TICK_DURATION {
            return;
        }
        self.last_tick = Instant::now();

        let new_head = self.snake.advance();

        // Check collision with self
        if self.snake.body.iter().skip(1).any(|&p| p == new_head) {
            self.game_over = true;
            println!("💀 Game Over! Final Score: {}", self.score);
            return;
        }

        // Check apple collision
        if new_head == self.apple {
            self.score += 1;
            self.spawn_apple();
            println!("🍎 Score: {}", self.score);
        } else {
            self.snake.shrink();
        }
    }

    fn spawn_apple(&mut self) {
        // Simple pseudo-random placement
        let seed = Instant::now().elapsed().as_nanos() as u32;
        self.apple = Position::new(
            (seed.wrapping_mul(7) ^ self.score.wrapping_mul(13)) % GRID_WIDTH as u32,
            (seed.wrapping_mul(11) ^ self.score.wrapping_mul(17)) % GRID_HEIGHT as u32,
        );
    }
}

// ============================================================================
// INPUT HANDLING
// ============================================================================

#[derive(Default)]
struct Input {
    direction: Option<Direction>,
    quit: bool,
}

impl Input {
    fn from_window(window: &Window) -> Self {
        let mut input = Input::default();

        if window.is_key_down(Key::Escape) {
            input.quit = true;
        }

        if window.is_key_down(Key::Up) {
            input.direction = Some(Direction::Up);
        } else if window.is_key_down(Key::Down) {
            input.direction = Some(Direction::Down);
        } else if window.is_key_down(Key::Left) {
            input.direction = Some(Direction::Left);
        } else if window.is_key_down(Key::Right) {
            input.direction = Some(Direction::Right);
        }

        input
    }
}

// ============================================================================
// GPU RENDERER - Hardware Abstraction
// ============================================================================

#[cube(launch)]
fn render_kernel(
    output: &mut Array<f32>,
    snake_data: &Array<u32>,
    snake_length: u32,
    apple_x: u32,
    apple_y: u32,
    width: u32,
    height: u32,
    cell_size: u32,
) {
    let pixel_index = ABSOLUTE_POS;
    
    if pixel_index < width * height {
        let x = pixel_index % width;
        let y = pixel_index / width;
        let grid_x = x / cell_size;
        let grid_y = y / cell_size;

        // Default: green background
        let mut r = 0.0;
        let mut g = 0.4;
        let mut b = 0.0;

        // Check apple
        if grid_x == apple_x && grid_y == apple_y {
            r = 0.9;
            g = 0.0;
            b = 0.0;
        } else {
            // Check snake
            let mut i = 0u32;
            loop {
                if i >= snake_length {
                    break;
                }
                
                let snake_x = snake_data[i * 2u32];
                let snake_y = snake_data[i * 2u32 + 1u32];
                
                if grid_x == snake_x && grid_y == snake_y {
                    r = 0.0;
                    g = 0.0;
                    b = 0.0;
                    break;
                }
                
                i += 1u32;
            }
        }

        let output_index = pixel_index * 3u32;
        output[output_index] = r;
        output[output_index + 1u32] = g;
        output[output_index + 2u32] = b;
    }
}

struct Renderer<R: Runtime> {
    client: ComputeClient<R::Server>,
    frame_buffer: Handle,
}

impl<R: Runtime> Renderer<R> {
    fn new(client: ComputeClient<R::Server>) -> Self {
        let frame_buffer = client.empty(SCREEN_WIDTH * SCREEN_HEIGHT * 3 * std::mem::size_of::<f32>());
        Self { client, frame_buffer }
    }

    fn render(&self, game: &GameState) -> Vec<u32> {
        let snake_data = game.snake.serialize();
        let snake_buffer = self.client.create(bytemuck::cast_slice(&snake_data));

        render_kernel::launch::<R>(
            &self.client,
            CubeCount::Static(((SCREEN_WIDTH * SCREEN_HEIGHT + 255) / 256) as u32, 1, 1),
            CubeDim::new(256, 1, 1),
            unsafe { ArrayArg::from_raw_parts::<f32>(&self.frame_buffer, SCREEN_WIDTH * SCREEN_HEIGHT * 3, 1) },
            unsafe { ArrayArg::from_raw_parts::<u32>(&snake_buffer, snake_data.len(), 1) },
            ScalarArg::new(game.snake.body.len() as u32),
            ScalarArg::new(game.apple.x),
            ScalarArg::new(game.apple.y),
            ScalarArg::new(SCREEN_WIDTH as u32),
            ScalarArg::new(SCREEN_HEIGHT as u32),
            ScalarArg::new(CELL_SIZE as u32),
        );

        pollster::block_on(self.client.sync());

        let data = self.client.read(vec![self.frame_buffer.clone()]);
        let rgb: &[f32] = bytemuck::cast_slice(&data[0]);

        self.convert_to_pixels(rgb)
    }

    fn convert_to_pixels(&self, rgb: &[f32]) -> Vec<u32> {
        (0..SCREEN_WIDTH * SCREEN_HEIGHT)
            .map(|i| {
                let r = (rgb[i * 3].clamp(0.0, 1.0) * 255.0) as u32;
                let g = (rgb[i * 3 + 1].clamp(0.0, 1.0) * 255.0) as u32;
                let b = (rgb[i * 3 + 2].clamp(0.0, 1.0) * 255.0) as u32;
                (r << 16) | (g << 8) | b
            })
            .collect()
    }
}

// ============================================================================
// MAIN - Top Level Module
// ============================================================================

fn main() {
    type Runtime = cubecl::cuda::CudaRuntime;
    
    let client = Runtime::client(&Default::default());
    let renderer = Renderer::<Runtime>::new(client);
    
    let mut window = Window::new(
        "🐍 Snake Game - GPU Accelerated",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions::default(),
    )
    .expect("Failed to create window");
    
    window.set_target_fps(TARGET_FPS as usize);

    let mut game = GameState::new();

    println!("╔════════════════════════════════════════╗");
    println!("║      🐍 SNAKE GAME - GPU Edition      ║");
    println!("╠════════════════════════════════════════╣");
    println!("║  Arrow Keys : Move                     ║");
    println!("║  ESC        : Quit                     ║");
    println!("╚════════════════════════════════════════╝");
    println!("\nScore: {}", game.score);

    while window.is_open() {
        let input = Input::from_window(&window);
        
        if input.quit {
            break;
        }

        game.handle_input(input);
        game.tick();

        let pixels = renderer.render(&game);
        
        window
            .update_with_buffer(&pixels, SCREEN_WIDTH, SCREEN_HEIGHT)
            .expect("Failed to update window");
    }

    println!("\n👋 Thanks for playing! Final Score: {}", game.score);
}
