use cubecl::prelude::*;
use cubecl::server::Handle;
use minifb::{Key, Window, WindowOptions};
use std::collections::{VecDeque, HashSet, BinaryHeap};
use std::cmp::Ordering;
use std::time::{Duration, Instant};

// ============================================================================
// CONSTANTS - Hardware Specification
// ============================================================================

const SCREEN_WIDTH: usize = 800;
const SCREEN_HEIGHT: usize = 600;
const CELL_SIZE: usize = 20;
const GRID_WIDTH: usize = SCREEN_WIDTH / CELL_SIZE;
const GRID_HEIGHT: usize = SCREEN_HEIGHT / CELL_SIZE;
const TICK_DURATION: Duration = Duration::from_millis(100);
const TARGET_FPS: u64 = 60;

// ============================================================================
// TYPES - Basic Building Blocks
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    fn manhattan_distance(&self, other: &Position) -> u32 {
        let dx = (self.x as i32 - other.x as i32).abs() as u32;
        let dy = (self.y as i32 - other.y as i32).abs() as u32;
        dx + dy
    }

    fn neighbors(&self) -> Vec<(Direction, Position)> {
        Direction::all()
            .iter()
            .map(|&dir| (dir, self.move_by(dir)))
            .collect()
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

    fn all() -> [Direction; 4] {
        [Direction::Up, Direction::Down, Direction::Left, Direction::Right]
    }
}

// ============================================================================
// A* PATHFINDING WITH SAFETY CHECKS
// ============================================================================

#[derive(Clone, Eq, PartialEq)]
struct Node {
    position: Position,
    g_cost: u32,
    h_cost: u32,
    parent: Option<Direction>,
}

impl Node {
    fn f_cost(&self) -> u32 {
        self.g_cost + self.h_cost
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost().cmp(&self.f_cost())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct AIAgent;

impl AIAgent {
    fn find_path(start: Position, goal: Position, obstacles: &HashSet<Position>) -> Option<Vec<Direction>> {
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();
        let mut came_from: std::collections::HashMap<Position, Direction> = std::collections::HashMap::new();

        open_set.push(Node {
            position: start,
            g_cost: 0,
            h_cost: start.manhattan_distance(&goal),
            parent: None,
        });

        while let Some(current) = open_set.pop() {
            if current.position == goal {
                let mut path = Vec::new();
                let mut pos = goal;
                while let Some(&dir) = came_from.get(&pos) {
                    path.push(dir);
                    pos = pos.move_by(dir.opposite());
                }
                path.reverse();
                return Some(path);
            }

            if closed_set.contains(&current.position) {
                continue;
            }
            closed_set.insert(current.position);

            for (direction, neighbor) in current.position.neighbors() {
                if obstacles.contains(&neighbor) || closed_set.contains(&neighbor) {
                    continue;
                }

                let g_cost = current.g_cost + 1;
                let h_cost = neighbor.manhattan_distance(&goal);

                came_from.insert(neighbor, direction);
                open_set.push(Node {
                    position: neighbor,
                    g_cost,
                    h_cost,
                    parent: Some(direction),
                });
            }
        }

        None
    }

    fn flood_fill(start: Position, obstacles: &HashSet<Position>) -> usize {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited.insert(start);

        while let Some(pos) = queue.pop_front() {
            for (_, neighbor) in pos.neighbors() {
                if !obstacles.contains(&neighbor) && !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back(neighbor);
                }
            }
        }

        visited.len()
    }

    fn is_safe_move(next_pos: Position, snake_body: &VecDeque<Position>) -> bool {
        let mut future_obstacles: HashSet<Position> = snake_body.iter().skip(1).copied().collect();
        
        // Simulate the move
        let reachable = Self::flood_fill(next_pos, &future_obstacles);
        
        // Need enough space for the snake to move
        reachable > snake_body.len()
    }

    fn find_safest_direction(head: Position, snake_body: &VecDeque<Position>, current_dir: Direction) -> Direction {
        let obstacles: HashSet<Position> = snake_body.iter().copied().collect();
        
        // Score each direction
        let mut best_dir = current_dir;
        let mut best_score = 0;

        for &dir in Direction::all().iter() {
            if dir == current_dir.opposite() {
                continue;
            }

            let next_pos = head.move_by(dir);
            
            if obstacles.contains(&next_pos) {
                continue;
            }

            let mut temp_obstacles = obstacles.clone();
            temp_obstacles.remove(snake_body.back().unwrap());
            temp_obstacles.insert(next_pos);
            
            let space = Self::flood_fill(next_pos, &temp_obstacles);
            
            if space > best_score {
                best_score = space;
                best_dir = dir;
            }
        }

        best_dir
    }

    fn decide(game: &GameState) -> Direction {
        let head = game.snake.head();
        let apple = game.apple;
        let snake_body = &game.snake.body;
        let current_dir = game.snake.direction;

        // Build obstacle set (snake body)
        let obstacles: HashSet<Position> = snake_body.iter().copied().collect();

        // Try to find path to apple
        if let Some(path) = Self::find_path(head, apple, &obstacles) {
            if let Some(&first_move) = path.first() {
                let next_pos = head.move_by(first_move);
                
                // Check if this move is safe (doesn't trap us)
                if Self::is_safe_move(next_pos, snake_body) {
                    return first_move;
                }
            }
        }

        // If no safe path to apple, find safest direction to maximize space
        Self::find_safest_direction(head, snake_body, current_dir)
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
        body.push_back(Position::new(head.x.saturating_sub(1), head.y));
        body.push_back(Position::new(head.x.saturating_sub(2), head.y));
        
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameMode {
    Human,
    AI,
}

struct GameState {
    snake: Snake,
    apple: Position,
    score: u32,
    game_over: bool,
    last_tick: Instant,
    mode: GameMode,
}

impl GameState {
    fn new(mode: GameMode) -> Self {
        Self {
            snake: Snake::new(Position::new(
                GRID_WIDTH as u32 / 2,
                GRID_HEIGHT as u32 / 2,
            )),
            apple: Position::new(10, 10),
            score: 0,
            game_over: false,
            last_tick: Instant::now(),
            mode,
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
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        loop {
            let pos = Position::new(
                rng.gen_range(0..GRID_WIDTH as u32),
                rng.gen_range(0..GRID_HEIGHT as u32),
            );
            
            if !self.snake.contains(pos) {
                self.apple = pos;
                break;
            }
        }
    }
}

// ============================================================================
// INPUT HANDLING
// ============================================================================

#[derive(Default)]
struct Input {
    direction: Option<Direction>,
    quit: bool,
    toggle_mode: bool,
}

impl Input {
    fn from_window(window: &Window) -> Self {
        let mut input = Input::default();

        if window.is_key_down(Key::Escape) {
            input.quit = true;
        }

        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            input.toggle_mode = true;
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
    is_ai: u32,
) {
    let pixel_index = ABSOLUTE_POS;
    
    if pixel_index < width * height {
        let x = pixel_index % width;
        let y = pixel_index / width;
        let grid_x = x / cell_size;
        let grid_y = y / cell_size;

        let mut r = 0.0;
        let mut g = 0.4;
        let mut b = 0.0;

        if grid_x == apple_x && grid_y == apple_y {
            r = 0.9;
            g = 0.0;
            b = 0.0;
        } else {
            let mut i = 0u32;
            loop {
                if i >= snake_length {
                    break;
                }
                
                let snake_x = snake_data[i * 2u32];
                let snake_y = snake_data[i * 2u32 + 1u32];
                
                if grid_x == snake_x && grid_y == snake_y {
                    if is_ai == 1u32 {
                        r = 0.0;
                        g = 0.5;
                        b = 0.9;
                    } else {
                        r = 0.0;
                        g = 0.0;
                        b = 0.0;
                    }
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
            ScalarArg::new(if game.mode == GameMode::AI { 1u32 } else { 0u32 }),
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
        "🐍 Snake - GPU + A* Pathfinding AI",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions::default(),
    )
    .expect("Failed to create window");
    
    window.set_target_fps(TARGET_FPS as usize);

    let mut game = GameState::new(GameMode::Human);

    println!("╔════════════════════════════════════════╗");
    println!("║   🐍 SNAKE - GPU + A* AI Algorithm    ║");
    println!("╠════════════════════════════════════════╣");
    println!("║  Arrow Keys : Move (Human Mode)       ║");
    println!("║  SPACE      : Toggle Human/AI         ║");
    println!("║  ESC        : Quit                     ║");
    println!("╠════════════════════════════════════════╣");
    println!("║  AI Strategy:                          ║");
    println!("║  • A* pathfinding to apple             ║");
    println!("║  • Flood-fill safety checks            ║");
    println!("║  • Space maximization fallback         ║");
    println!("╚════════════════════════════════════════╝");
    println!("\n👤 Mode: HUMAN | Score: {}", game.score);

    while window.is_open() {
        let input = Input::from_window(&window);
        
        if input.quit {
            break;
        }

        if input.toggle_mode {
            game.mode = match game.mode {
                GameMode::Human => {
                    println!("\n🤖 Switched to AI Mode (A* Pathfinding)");
                    GameMode::AI
                }
                GameMode::AI => {
                    println!("\n👤 Switched to Human Mode");
                    GameMode::Human
                }
            };
            game = GameState::new(game.mode);
        }

        match game.mode {
            GameMode::Human => {
                game.handle_input(input);
            }
            GameMode::AI => {
                if game.last_tick.elapsed() >= TICK_DURATION {
                    let action = AIAgent::decide(&game);
                    game.snake.set_direction(action);
                }
            }
        }

        game.tick();
        let pixels = renderer.render(&game);
        
        window
            .update_with_buffer(&pixels, SCREEN_WIDTH, SCREEN_HEIGHT)
            .expect("Failed to update window");
    }

    println!("\n👋 Thanks for playing! Final Score: {}", game.score);
}
