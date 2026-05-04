use std::cmp::{max, min};
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::{Duration, Instant};

const EMPTY: u8 = 0;
const X: u8 = 1;
const O: u8 = 2;
const DRAW: u8 = 3;
const ANY_BOARD: i8 = -1;

const MAX_PLY: usize = 128;
const MAX_MOVES: usize = 81;
const INF: i32 = 1_000_000_000;
const WIN_SCORE: i32 = 50_000_000;

const TT_BITS: usize = 20;
const TT_SIZE: usize = 1 << TT_BITS;
const TT_MASK: usize = TT_SIZE - 1;

const MACRO_CENTER_WEIGHT: i32 = 1344;
const MACRO_CORNER_WEIGHT: i32 = 361;
const MACRO_EDGE_WEIGHT: i32 = 420;
const LOCAL_WIN_WEIGHT: i32 = 2816;
const LOCAL_CENTER_WEIGHT: i32 = 33;
const LOCAL_CORNER_WEIGHT: i32 = 30;
const LOCAL_EDGE_WEIGHT: i32 = 12;
const LOCAL_TWO_WEIGHT: i32 = 260;
const LOCAL_ONE_WEIGHT: i32 = 28;
const LOCAL_BLOCK_TWO_WEIGHT: i32 = 97;
const DESTINATION_WEIGHT: i32 = 52;
const MOBILITY_WEIGHT: i32 = 3;
const CLOSED_BOARD_PENALTY: i32 = 76;
const TUNABLE_WEIGHT_COUNT: usize = 13;

const LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

#[derive(Clone, Copy, Debug)]
struct EvalWeights {
    macro_center: i32,
    macro_corner: i32,
    macro_edge: i32,
    local_win: i32,
    local_center: i32,
    local_corner: i32,
    local_edge: i32,
    local_two: i32,
    local_one: i32,
    local_block_two: i32,
    destination: i32,
    mobility: i32,
    closed_board_penalty: i32,
}

impl EvalWeights {
    const fn default() -> Self {
        Self {
            macro_center: MACRO_CENTER_WEIGHT,
            macro_corner: MACRO_CORNER_WEIGHT,
            macro_edge: MACRO_EDGE_WEIGHT,
            local_win: LOCAL_WIN_WEIGHT,
            local_center: LOCAL_CENTER_WEIGHT,
            local_corner: LOCAL_CORNER_WEIGHT,
            local_edge: LOCAL_EDGE_WEIGHT,
            local_two: LOCAL_TWO_WEIGHT,
            local_one: LOCAL_ONE_WEIGHT,
            local_block_two: LOCAL_BLOCK_TWO_WEIGHT,
            destination: DESTINATION_WEIGHT,
            mobility: MOBILITY_WEIGHT,
            closed_board_penalty: CLOSED_BOARD_PENALTY,
        }
    }

    fn global_position_weight(&self, board_idx: usize) -> i32 {
        match board_idx {
            4 => self.macro_center,
            0 | 2 | 6 | 8 => self.macro_corner,
            _ => self.macro_edge,
        }
    }

    fn local_position_weight(&self, local_cell: usize) -> i32 {
        match local_cell {
            4 => self.local_center,
            0 | 2 | 6 | 8 => self.local_corner,
            _ => self.local_edge,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Undo {
    idx: u8,
    prev_next_board: i8,
    prev_local_status: u8,
    prev_global_status: u8,
    prev_hash: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TtEntry {
    key: u64,
    score: i32,
    depth: i16,
    flag: u8,
    best_move: u8,
}

impl Default for TtEntry {
    fn default() -> Self {
        Self {
            key: 0,
            score: 0,
            depth: -1,
            flag: 0,
            best_move: 255,
        }
    }
}

#[derive(Clone, Copy)]
struct SearchResult {
    best_move: Option<u8>,
    score: i32,
    depth: i32,
    completed: bool,
}

#[derive(Clone, Copy, Default)]
struct GameStats {
    winner: u8,
    moves: usize,
    elapsed_ms: u128,
    total_nodes: u64,
    searched_moves: usize,
    completed_searches: usize,
    time_cutoffs: usize,
    depth_sum: i64,
    score_sum: i64,
}

impl GameStats {
    fn record_search(&mut self, result: SearchResult, nodes: u64) {
        self.total_nodes += nodes;
        self.searched_moves += 1;
        self.depth_sum += result.depth as i64;
        self.score_sum += result.score as i64;
        if result.completed {
            self.completed_searches += 1;
        } else {
            self.time_cutoffs += 1;
        }
    }

    fn avg_depth(&self) -> f64 {
        if self.searched_moves == 0 {
            0.0
        } else {
            self.depth_sum as f64 / self.searched_moves as f64
        }
    }

    fn avg_score(&self) -> f64 {
        if self.searched_moves == 0 {
            0.0
        } else {
            self.score_sum as f64 / self.searched_moves as f64
        }
    }
}

#[derive(Clone)]
struct Zobrist {
    piece: [[u64; 2]; 81],
    next_board: [u64; 10],
    side_to_move: u64,
}

impl Zobrist {
    fn new() -> Self {
        fn splitmix64(state: &mut u64) -> u64 {
            *state = state.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = *state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }

        let mut seed = 0x1234_5678_9ABC_DEF0;
        let mut piece = [[0u64; 2]; 81];
        let mut next_board = [0u64; 10];
        for entry in &mut piece {
            entry[0] = splitmix64(&mut seed);
            entry[1] = splitmix64(&mut seed);
        }
        for key in &mut next_board {
            *key = splitmix64(&mut seed);
        }
        let side_to_move = splitmix64(&mut seed);
        Self {
            piece,
            next_board,
            side_to_move,
        }
    }
}

#[derive(Clone)]
struct Board {
    cells: [u8; 81],
    local_status: [u8; 9],
    global_status: u8,
    current_player: u8,
    next_board: i8,
    ply: usize,
    hash: u64,
    zobrist: Zobrist,
}

impl Board {
    fn new() -> Self {
        let zobrist = Zobrist::new();
        let mut board = Self {
            cells: [EMPTY; 81],
            local_status: [EMPTY; 9],
            global_status: EMPTY,
            current_player: X,
            next_board: ANY_BOARD,
            ply: 0,
            hash: 0,
            zobrist,
        };
        board.hash = board.compute_hash();
        board
    }

    fn reset(&mut self, starter: u8) {
        self.cells = [EMPTY; 81];
        self.local_status = [EMPTY; 9];
        self.global_status = EMPTY;
        self.current_player = starter;
        self.next_board = ANY_BOARD;
        self.ply = 0;
        self.hash = self.compute_hash();
    }

    fn compute_hash(&self) -> u64 {
        let mut h = 0u64;
        for idx in 0..81 {
            match self.cells[idx] {
                X => h ^= self.zobrist.piece[idx][0],
                O => h ^= self.zobrist.piece[idx][1],
                _ => {}
            }
        }
        let nb_idx = if self.next_board == ANY_BOARD {
            9
        } else {
            self.next_board as usize
        };
        h ^= self.zobrist.next_board[nb_idx];
        if self.current_player == O {
            h ^= self.zobrist.side_to_move;
        }
        h
    }

    #[inline]
    fn other(player: u8) -> u8 {
        if player == X { O } else { X }
    }

    #[inline]
    fn cell_coords(idx: usize) -> (usize, usize) {
        (idx % 9, idx / 9)
    }

    #[inline]
    fn local_board_of(idx: usize) -> usize {
        let (x, y) = Self::cell_coords(idx);
        (y / 3) * 3 + (x / 3)
    }

    #[inline]
    fn cell_in_local(idx: usize) -> usize {
        let (x, y) = Self::cell_coords(idx);
        (y % 3) * 3 + (x % 3)
    }

    #[inline]
    fn global_to_index(col: usize, row: usize) -> usize {
        row * 9 + col
    }

    #[inline]
    fn local_cell_index(board_idx: usize, local_cell: usize) -> usize {
        let bx = board_idx % 3;
        let by = board_idx / 3;
        let lx = local_cell % 3;
        let ly = local_cell / 3;
        (by * 3 + ly) * 9 + (bx * 3 + lx)
    }

    fn detect_local_status(&self, board_idx: usize) -> u8 {
        for line in LINES {
            let a = self.cells[Self::local_cell_index(board_idx, line[0])];
            if a != EMPTY
                && a == self.cells[Self::local_cell_index(board_idx, line[1])]
                && a == self.cells[Self::local_cell_index(board_idx, line[2])]
            {
                return a;
            }
        }
        for i in 0..9 {
            if self.cells[Self::local_cell_index(board_idx, i)] == EMPTY {
                return EMPTY;
            }
        }
        DRAW
    }

    fn detect_global_status(&self) -> u8 {
        for line in LINES {
            let a = self.local_status[line[0]];
            if a != EMPTY && a != DRAW && a == self.local_status[line[1]] && a == self.local_status[line[2]] {
                return a;
            }
        }
        for &status in &self.local_status {
            if status == EMPTY {
                return EMPTY;
            }
        }
        DRAW
    }

    fn is_legal(&self, idx: usize) -> bool {
        if idx >= 81 || self.cells[idx] != EMPTY || self.global_status != EMPTY {
            return false;
        }
        let board_idx = Self::local_board_of(idx);
        if self.local_status[board_idx] != EMPTY {
            return false;
        }
        if self.next_board == ANY_BOARD {
            return true;
        }
        board_idx == self.next_board as usize
    }

    fn generate_moves(&self, out: &mut [u8; MAX_MOVES]) -> usize {
        if self.global_status != EMPTY {
            return 0;
        }

        let mut len = 0usize;
        if self.next_board != ANY_BOARD {
            let board_idx = self.next_board as usize;
            if self.local_status[board_idx] == EMPTY {
                for local_cell in 0..9 {
                    let idx = Self::local_cell_index(board_idx, local_cell);
                    if self.cells[idx] == EMPTY {
                        out[len] = idx as u8;
                        len += 1;
                    }
                }
                return len;
            }
        }

        for board_idx in 0..9 {
            if self.local_status[board_idx] != EMPTY {
                continue;
            }
            for local_cell in 0..9 {
                let idx = Self::local_cell_index(board_idx, local_cell);
                if self.cells[idx] == EMPTY {
                    out[len] = idx as u8;
                    len += 1;
                }
            }
        }
        len
    }

    fn apply_move(&mut self, idx: u8) -> Undo {
        let idx_usize = idx as usize;
        let board_idx = Self::local_board_of(idx_usize);
        let destination = Self::cell_in_local(idx_usize);
        let undo = Undo {
            idx,
            prev_next_board: self.next_board,
            prev_local_status: self.local_status[board_idx],
            prev_global_status: self.global_status,
            prev_hash: self.hash,
        };

        self.hash ^= self.zobrist.next_board[if self.next_board == ANY_BOARD {
            9
        } else {
            self.next_board as usize
        }];
        if self.current_player == O {
            self.hash ^= self.zobrist.side_to_move;
        }

        self.cells[idx_usize] = self.current_player;
        self.hash ^= self.zobrist.piece[idx_usize][(self.current_player - 1) as usize];

        self.local_status[board_idx] = self.detect_local_status(board_idx);
        self.global_status = self.detect_global_status();

        self.next_board = if self.local_status[destination] == EMPTY {
            destination as i8
        } else {
            ANY_BOARD
        };

        self.current_player = Self::other(self.current_player);
        if self.current_player == O {
            self.hash ^= self.zobrist.side_to_move;
        }
        self.hash ^= self.zobrist.next_board[if self.next_board == ANY_BOARD {
            9
        } else {
            self.next_board as usize
        }];
        self.ply += 1;
        undo
    }

    fn undo_move(&mut self, undo: Undo) {
        self.ply -= 1;
        self.current_player = Self::other(self.current_player);
        self.cells[undo.idx as usize] = EMPTY;
        let board_idx = Self::local_board_of(undo.idx as usize);
        self.local_status[board_idx] = undo.prev_local_status;
        self.global_status = undo.prev_global_status;
        self.next_board = undo.prev_next_board;
        self.hash = undo.prev_hash;
    }

    fn immediate_local_wins_mask(&self, board_idx: usize, player: u8) -> u16 {
        if self.local_status[board_idx] != EMPTY {
            return 0;
        }
        let mut mask = 0u16;
        for line in LINES {
            let mut count_player = 0;
            let mut empty_cell = None;
            let mut blocked = false;
            for &cell in &line {
                let idx = Self::local_cell_index(board_idx, cell);
                match self.cells[idx] {
                    p if p == player => count_player += 1,
                    EMPTY => empty_cell = Some(cell),
                    _ => blocked = true,
                }
            }
            if !blocked && count_player == 2 {
                if let Some(cell) = empty_cell {
                    mask |= 1 << cell;
                }
            }
        }
        mask
    }

    fn local_board_feature_value(&self, board_idx: usize, player: u8, weights: &EvalWeights) -> i32 {
        let opp = Self::other(player);
        match self.local_status[board_idx] {
            p if p == player => return weights.local_win,
            p if p == opp => return -weights.local_win,
            DRAW => return -weights.closed_board_penalty,
            _ => {}
        }

        let mut score = 0i32;
        for local_cell in 0..9 {
            let idx = Self::local_cell_index(board_idx, local_cell);
            let position_weight = weights.local_position_weight(local_cell);
            match self.cells[idx] {
                p if p == player => score += position_weight,
                p if p == opp => score -= position_weight,
                _ => {}
            }
        }

        for line in LINES {
            let mut my_count = 0;
            let mut opp_count = 0;
            let mut empty_count = 0;
            for &cell in &line {
                let idx = Self::local_cell_index(board_idx, cell);
                match self.cells[idx] {
                    p if p == player => my_count += 1,
                    p if p == opp => opp_count += 1,
                    _ => empty_count += 1,
                }
            }
            if opp_count == 0 {
                if my_count == 2 && empty_count == 1 {
                    score += weights.local_two;
                } else if my_count == 1 && empty_count == 2 {
                    score += weights.local_one;
                }
            }
            if my_count == 0 && opp_count == 2 && empty_count == 1 {
                score -= weights.local_block_two;
            }
        }

        score
    }

    fn macro_line_value(&self, player: u8) -> i32 {
        let opp = Self::other(player);
        let mut score = 0;
        for line in LINES {
            let mut mine = 0;
            let mut theirs = 0;
            let mut open = 0;
            for &b in &line {
                match self.local_status[b] {
                    p if p == player => mine += 1,
                    p if p == opp => theirs += 1,
                    EMPTY => open += 1,
                    DRAW => {}
                    _ => {}
                }
            }
            if theirs == 0 {
                score += match (mine, open) {
                    (3, _) => WIN_SCORE / 2,
                    (2, 1) => 18_000,
                    (1, 2) => 2_400,
                    _ => 280,
                };
            }
            if mine == 0 {
                score -= match (theirs, open) {
                    (3, _) => WIN_SCORE / 2,
                    (2, 1) => 19_000,
                    (1, 2) => 2_600,
                    _ => 300,
                };
            }
        }
        score
    }

    fn forced_destination_term(&self, weights: &EvalWeights) -> i32 {
        if self.next_board == ANY_BOARD {
            return 0;
        }
        let board_idx = self.next_board as usize;
        if self.local_status[board_idx] != EMPTY {
            return 0;
        }
        let side = self.current_player;
        let side_sign = if side == X { 1 } else { -1 };
        let own = self.local_board_feature_value(board_idx, side, weights);
        let opp = self.local_board_feature_value(board_idx, Self::other(side), weights);
        let mobility = self.board_mobility(board_idx);
        side_sign * ((own - opp / 2) * weights.destination / 100 - mobility * weights.mobility)
    }

    fn board_mobility(&self, board_idx: usize) -> i32 {
        let mut count = 0;
        for local_cell in 0..9 {
            let idx = Self::local_cell_index(board_idx, local_cell);
            if self.cells[idx] == EMPTY {
                count += 1;
            }
        }
        count
    }

    fn evaluate_absolute(&self, weights: &EvalWeights) -> i32 {
        match self.global_status {
            X => return WIN_SCORE - self.ply as i32,
            O => return -WIN_SCORE + self.ply as i32,
            DRAW => return 0,
            _ => {}
        }

        let mut score = 0i32;
        for b in 0..9 {
            let global_weight = weights.global_position_weight(b);
            match self.local_status[b] {
                X => score += weights.local_win + global_weight,
                O => score -= weights.local_win + global_weight,
                DRAW => {}
                _ => {
                    score += self.local_board_feature_value(b, X, weights);
                    score -= self.local_board_feature_value(b, O, weights);
                }
            }
        }

        score += self.macro_line_value(X);
        score -= self.macro_line_value(O);
        score += self.forced_destination_term(weights);
        score
    }

    fn evaluate_for_side_to_move(&self, weights: &EvalWeights) -> i32 {
        let absolute = self.evaluate_absolute(weights);
        if self.current_player == X {
            absolute
        } else {
            -absolute
        }
    }

    fn display(&self, last_move: Option<u8>) {
        println!();
        if let Some(mv) = last_move {
            let (x, y) = Self::cell_coords(mv as usize);
            println!("Last move: col {}, row {}", x + 1, y + 1);
        }
        match self.next_board {
            ANY_BOARD => println!("Target board: any open local board"),
            b => println!("Target board: local board {}", b + 1),
        }
        println!("Player to move: {}", if self.current_player == X { 'X' } else { 'O' });
        println!();

        for row in 0..9 {
            if row > 0 && row % 3 == 0 {
                println!("===========+===========+===========");
            }
            for col in 0..9 {
                if col > 0 && col % 3 == 0 {
                    print!(" |");
                }
                let idx = Self::global_to_index(col, row);
                let ch = match self.cells[idx] {
                    X => 'X',
                    O => 'O',
                    _ => '.',
                };
                print!(" {}", ch);
            }
            println!();
        }
        println!();
        println!("Macro board:");
        for row in 0..3 {
            for col in 0..3 {
                let status = self.local_status[row * 3 + col];
                let ch = match status {
                    X => 'X',
                    O => 'O',
                    DRAW => '#',
                    _ => '.',
                };
                print!(" {}", ch);
            }
            println!();
        }
        println!();
    }
}

struct Searcher {
    weights: EvalWeights,
    tt: Vec<TtEntry>,
    killer_moves: [[u8; 2]; MAX_PLY],
    history: [[i32; 81]; 2],
    nodes: u64,
    deadline: Option<Instant>,
    timed_out: bool,
    root_best_move: Option<u8>,
    root_best_score: i32,
}

#[derive(Clone, Copy)]
enum SideKind {
    Human,
    Ai,
}

#[derive(Clone, Copy)]
struct GameConfig {
    x_side: SideKind,
    o_side: SideKind,
    starter: u8,
    depth_limit: i32,
    time_limit_ms: Option<u64>,
}

fn flush_stdout() {
    io::stdout().flush().expect("failed to flush stdout");
}

fn read_line() -> String {
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => return input.trim().to_string(),
            Err(_) => println!("Input error. Please try again."),
        }
    }
}

fn prompt_choice(prompt: &str, valid: &[&str]) -> String {
    loop {
        print!("{prompt}");
        flush_stdout();
        let input = read_line();
        let lowered = input.to_lowercase();
        if valid.iter().any(|v| *v == lowered) {
            return lowered;
        }
        println!("Invalid choice. Expected one of: {}", valid.join(", "));
    }
}

fn prompt_u64(prompt: &str, min_value: u64, max_value: u64) -> u64 {
    loop {
        print!("{prompt}");
        flush_stdout();
        let input = read_line();
        if let Ok(value) = input.parse::<u64>() {
            if value >= min_value && value <= max_value {
                return value;
            }
        }
        println!("Please enter an integer in [{min_value}, {max_value}].");
    }
}

fn prompt_game_config() -> GameConfig {
    println!("Ultimate Tic Tac Toe");
    println!("Modes: human vs ai, ai vs ai");
    let mode = prompt_choice("Select mode (`h` for human vs ai, `a` for ai vs ai): ", &["h", "a"]);

    let starter_choice = prompt_choice("Who starts (`x` or `o`)? ", &["x", "o"]);
    let starter = if starter_choice == "x" { X } else { O };

    let depth_limit = prompt_u64("Depth limit per move (1-20 recommended): ", 1, 50) as i32;
    let time_limit_raw = prompt_u64("Time per move in milliseconds (0 for depth-only search): ", 0, 600_000);
    let time_limit_ms = if time_limit_raw == 0 {
        None
    } else {
        Some(time_limit_raw)
    };

    match mode.as_str() {
        "h" => {
            let human_side = prompt_choice("Human side (`x` or `o`): ", &["x", "o"]);
            let x_side = if human_side == "x" {
                SideKind::Human
            } else {
                SideKind::Ai
            };
            let o_side = if human_side == "o" {
                SideKind::Human
            } else {
                SideKind::Ai
            };
            GameConfig {
                x_side,
                o_side,
                starter,
                depth_limit,
                time_limit_ms,
            }
        }
        _ => GameConfig {
            x_side: SideKind::Ai,
            o_side: SideKind::Ai,
            starter,
            depth_limit,
            time_limit_ms,
        },
    }
}

fn side_kind_for_player(config: &GameConfig, player: u8) -> SideKind {
    if player == X {
        config.x_side
    } else {
        config.o_side
    }
}

fn prompt_human_move(board: &Board) -> u8 {
    loop {
        print!("Enter move as `column row` (1-9 1-9): ");
        flush_stdout();
        let input = read_line();
        let parts: Vec<_> = input.split_whitespace().collect();
        if parts.len() != 2 {
            println!("Please enter exactly two numbers.");
            continue;
        }
        let col = parts[0].parse::<usize>();
        let row = parts[1].parse::<usize>();
        match (col, row) {
            (Ok(c), Ok(r)) if (1..=9).contains(&c) && (1..=9).contains(&r) => {
                let idx = Board::global_to_index(c - 1, r - 1);
                if board.is_legal(idx) {
                    return idx as u8;
                }
                println!("Illegal move under current destination rules.");
            }
            _ => println!("Coordinates must be integers from 1 to 9."),
        }
    }
}

fn announce_result(board: &Board) {
    match board.global_status {
        X => println!("Game over: X wins the global board."),
        O => println!("Game over: O wins the global board."),
        DRAW => println!("Game over: draw."),
        _ => {}
    }
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

fn random_legal_move(board: &Board, seed: &mut u64) -> u8 {
    let mut moves = [0u8; MAX_MOVES];
    let len = board.generate_moves(&mut moves);
    let pick = (splitmix64(seed) as usize) % len;
    moves[pick]
}

fn player_name(player: u8) -> &'static str {
    match player {
        X => "X",
        O => "O",
        DRAW => "DRAW",
        _ => "OPEN",
    }
}

fn play_silent_game(
    x_weights: EvalWeights,
    o_weights: EvalWeights,
    starter: u8,
    depth_limit: i32,
    time_limit: Option<Duration>,
    seed: u64,
    random_opening_plies: usize,
) -> u8 {
    let mut board = Board::new();
    board.reset(starter);

    let mut searcher_x = Searcher::with_weights(x_weights);
    let mut searcher_o = Searcher::with_weights(o_weights);
    let mut rng = seed;

    while board.global_status == EMPTY {
        let mv = if board.ply < random_opening_plies {
            random_legal_move(&board, &mut rng)
        } else {
            let searcher = if board.current_player == X {
                &mut searcher_x
            } else {
                &mut searcher_o
            };
            let result = searcher.search_best_move(&mut board, depth_limit, time_limit);
            result.best_move.unwrap_or_else(|| {
                let mut moves = [0u8; MAX_MOVES];
                let len = board.generate_moves(&mut moves);
                moves[0.min(len.saturating_sub(1))]
            })
        };
        board.apply_move(mv);
    }

    board.global_status
}

fn play_profile_game_with_stats(
    x_weights: EvalWeights,
    o_weights: EvalWeights,
    starter: u8,
    depth_limit: i32,
    time_limit: Option<Duration>,
    seed: u64,
    random_opening_plies: usize,
) -> GameStats {
    let started = Instant::now();
    let mut board = Board::new();
    board.reset(starter);

    let mut searcher_x = Searcher::with_weights(x_weights);
    let mut searcher_o = Searcher::with_weights(o_weights);
    let mut rng = seed;
    let mut stats = GameStats::default();

    while board.global_status == EMPTY {
        let mv = if board.ply < random_opening_plies {
            random_legal_move(&board, &mut rng)
        } else {
            let searcher = if board.current_player == X {
                &mut searcher_x
            } else {
                &mut searcher_o
            };
            let result = searcher.search_best_move(&mut board, depth_limit, time_limit);
            let nodes = searcher.nodes;
            stats.record_search(result, nodes);
            result.best_move.unwrap_or_else(|| random_legal_move(&board, &mut rng))
        };
        board.apply_move(mv);
    }

    stats.winner = board.global_status;
    stats.moves = board.ply;
    stats.elapsed_ms = started.elapsed().as_millis();
    stats
}

fn run_game(config: GameConfig) {
    let mut board = Board::new();
    board.reset(config.starter);

    let mut searcher_x = Searcher::new();
    let mut searcher_o = Searcher::new();
    searcher_x.clear_for_new_game();
    searcher_o.clear_for_new_game();

    let mut last_move = None;
    board.display(last_move);

    while board.global_status == EMPTY {
        let player = board.current_player;
        let kind = side_kind_for_player(&config, player);

        let mv = match kind {
            SideKind::Human => prompt_human_move(&board),
            SideKind::Ai => {
                let time_limit = config.time_limit_ms.map(Duration::from_millis);
                let searcher = if player == X {
                    &mut searcher_x
                } else {
                    &mut searcher_o
                };
                let started = Instant::now();
                let result = searcher.search_best_move(&mut board, config.depth_limit, time_limit);
                let chosen = result.best_move.unwrap_or_else(|| {
                    let mut moves = [0u8; MAX_MOVES];
                    let len = board.generate_moves(&mut moves);
                    moves[0.min(len.saturating_sub(1))]
                });
                let (x, y) = Board::cell_coords(chosen as usize);
                println!(
                    "AI {} plays: col {}, row {} | score {} | depth {} | nodes {} | time {} ms{}",
                    if player == X { 'X' } else { 'O' },
                    x + 1,
                    y + 1,
                    result.score,
                    result.depth,
                    searcher.nodes,
                    started.elapsed().as_millis(),
                    if result.completed { "" } else { " (time cutoff)" }
                );
                chosen
            }
        };

        board.apply_move(mv);
        last_move = Some(mv);
        board.display(last_move);
    }

    announce_result(&board);
}

fn scaled(value: i32, percent: i32) -> i32 {
    ((value * percent) / 100).max(1)
}

fn mutate_weight(base: EvalWeights, field: usize, percent: i32) -> EvalWeights {
    let mut w = base;
    match field {
        0 => w.macro_center = scaled(w.macro_center, percent),
        1 => w.macro_corner = scaled(w.macro_corner, percent),
        2 => w.macro_edge = scaled(w.macro_edge, percent),
        3 => w.local_win = scaled(w.local_win, percent),
        4 => w.local_center = scaled(w.local_center, percent),
        5 => w.local_corner = scaled(w.local_corner, percent),
        6 => w.local_edge = scaled(w.local_edge, percent),
        7 => w.local_two = scaled(w.local_two, percent),
        8 => w.local_one = scaled(w.local_one, percent),
        9 => w.local_block_two = scaled(w.local_block_two, percent),
        10 => w.destination = scaled(w.destination, percent),
        11 => w.mobility = scaled(w.mobility, percent),
        12 => w.closed_board_penalty = scaled(w.closed_board_penalty, percent),
        _ => {}
    }
    w
}

fn random_mutation(base: EvalWeights, seed: &mut u64, step_percent: i32) -> EvalWeights {
    let mut weights = base;
    let fields_to_change = 1 + (splitmix64(seed) as usize % 4);
    for _ in 0..fields_to_change {
        let field = splitmix64(seed) as usize % TUNABLE_WEIGHT_COUNT;
        let span = step_percent.max(2);
        let delta = splitmix64(seed) as i32 % (span * 2 + 1) - span;
        weights = mutate_weight(weights, field, 100 + delta);
    }
    weights
}

fn candidate_variants(base: EvalWeights, step_percent: i32) -> Vec<EvalWeights> {
    let mut variants = Vec::with_capacity(43);
    variants.push(base);
    for field in 0..TUNABLE_WEIGHT_COUNT {
        variants.push(mutate_weight(base, field, 100 + step_percent));
        variants.push(mutate_weight(base, field, 100 - step_percent));
    }
    let mut seed = 0x7157_A1B0_5EED ^ step_percent as u64;
    for _ in 0..16 {
        variants.push(random_mutation(base, &mut seed, step_percent));
    }
    variants
}

fn compare_weights(
    candidate: EvalWeights,
    baseline: EvalWeights,
    games: usize,
    depth_limit: i32,
    time_limit: Option<Duration>,
    seed_base: u64,
) -> i32 {
    let mut score = 0;
    for g in 0..games {
        let starter = if g % 2 == 0 { X } else { O };
        let seed = seed_base ^ ((g as u64 + 1) * 0x9E37_79B9);

        let result_as_x = play_silent_game(
            candidate,
            baseline,
            starter,
            depth_limit,
            time_limit,
            seed,
            1 + (seed as usize % 5),
        );
        score += match result_as_x {
            X => 2,
            O => -2,
            _ => 0,
        };

        let result_as_o = play_silent_game(
            baseline,
            candidate,
            starter,
            depth_limit,
            time_limit,
            seed ^ 0xA5A5_5A5A_D3C1_B2E0,
            1 + ((seed >> 8) as usize % 5),
        );
        score += match result_as_o {
            O => 2,
            X => -2,
            _ => 0,
        };
    }
    score
}

fn print_weights(weights: EvalWeights) {
    println!("Best weights found:");
    println!("macro_center = {}", weights.macro_center);
    println!("macro_corner = {}", weights.macro_corner);
    println!("macro_edge = {}", weights.macro_edge);
    println!("local_win = {}", weights.local_win);
    println!("local_center = {}", weights.local_center);
    println!("local_corner = {}", weights.local_corner);
    println!("local_edge = {}", weights.local_edge);
    println!("local_two = {}", weights.local_two);
    println!("local_one = {}", weights.local_one);
    println!("local_block_two = {}", weights.local_block_two);
    println!("destination = {}", weights.destination);
    println!("mobility = {}", weights.mobility);
    println!("closed_board_penalty = {}", weights.closed_board_penalty);
}

fn write_weights_csv_columns(writer: &mut BufWriter<File>, weights: EvalWeights) -> io::Result<()> {
    write!(
        writer,
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        weights.macro_center,
        weights.macro_corner,
        weights.macro_edge,
        weights.local_win,
        weights.local_center,
        weights.local_corner,
        weights.local_edge,
        weights.local_two,
        weights.local_one,
        weights.local_block_two,
        weights.destination,
        weights.mobility,
        weights.closed_board_penalty
    )
}

#[derive(Clone, Copy)]
struct ProfileStanding {
    id: usize,
    weights: EvalWeights,
    points: i32,
    wins: usize,
    draws: usize,
    losses: usize,
    games: usize,
}

impl ProfileStanding {
    fn new(id: usize, weights: EvalWeights) -> Self {
        Self {
            id,
            weights,
            points: 0,
            wins: 0,
            draws: 0,
            losses: 0,
            games: 0,
        }
    }

    fn record(&mut self, result: i32) {
        self.games += 1;
        match result {
            1 => {
                self.wins += 1;
                self.points += 3;
            }
            0 => {
                self.draws += 1;
                self.points += 1;
            }
            _ => {
                self.losses += 1;
            }
        }
    }
}

fn run_benchmark(args: &[String]) -> io::Result<()> {
    let games = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
    let depth = args.get(3).and_then(|s| s.parse::<i32>().ok()).unwrap_or(3);
    let ms = args.get(4).and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
    let output = args.get(5).map(String::as_str).unwrap_or("benchmark.csv");
    let time_limit = if ms == 0 {
        None
    } else {
        Some(Duration::from_millis(ms))
    };

    let file = File::create(output)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "game,starter,winner,moves,depth,time_ms,random_opening_plies,elapsed_ms,total_nodes,searched_moves,avg_depth,completed_searches,time_cutoffs,avg_score,nodes_per_ms"
    )?;

    let mut x_wins = 0usize;
    let mut o_wins = 0usize;
    let mut draws = 0usize;
    let started_all = Instant::now();

    for game in 0..games {
        let starter = if game % 2 == 0 { X } else { O };
        let random_opening_plies = game % 6;
        let seed = 0xBEE5_0000_u64 ^ game as u64;
        let stats = play_profile_game_with_stats(
            EvalWeights::default(),
            EvalWeights::default(),
            starter,
            depth,
            time_limit,
            seed,
            random_opening_plies,
        );

        match stats.winner {
            X => x_wins += 1,
            O => o_wins += 1,
            DRAW => draws += 1,
            _ => {}
        }

        let nodes_per_ms = if stats.elapsed_ms == 0 {
            stats.total_nodes as f64
        } else {
            stats.total_nodes as f64 / stats.elapsed_ms as f64
        };
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{:.2},{},{},{:.2},{:.2}",
            game + 1,
            player_name(starter),
            player_name(stats.winner),
            stats.moves,
            depth,
            ms,
            random_opening_plies,
            stats.elapsed_ms,
            stats.total_nodes,
            stats.searched_moves,
            stats.avg_depth(),
            stats.completed_searches,
            stats.time_cutoffs,
            stats.avg_score(),
            nodes_per_ms
        )?;
    }

    writer.flush()?;
    println!(
        "Benchmark complete: games={games}, X wins={x_wins}, O wins={o_wins}, draws={draws}, elapsed={} ms, csv={output}",
        started_all.elapsed().as_millis()
    );
    Ok(())
}

fn run_training(args: &[String]) {
    let rounds = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(2);
    let games = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(4);
    let depth = args.get(4).and_then(|s| s.parse::<i32>().ok()).unwrap_or(3);
    let ms = args.get(5).and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
    let time_limit = if ms == 0 {
        None
    } else {
        Some(Duration::from_millis(ms))
    };

    let mut best = EvalWeights::default();
    let mut step = 18;

    println!(
        "Training: rounds={rounds}, games_per_candidate_pair={games}, depth={depth}, time_ms={ms}"
    );

    for round in 0..rounds {
        let variants = candidate_variants(best, step);
        let mut round_best = best;
        let mut round_score = i32::MIN;

        for (idx, candidate) in variants.iter().enumerate() {
            let score = compare_weights(
                *candidate,
                best,
                games,
                depth,
                time_limit,
                0xC0DE_0000 ^ ((round as u64) << 32) ^ idx as u64,
            );
            if score > round_score {
                round_score = score;
                round_best = *candidate;
            }
        }

        best = round_best;
        println!("Round {}: score {}, step {}%", round + 1, round_score, step);
        print_weights(best);
        step = (step * 2 / 3).max(5);
    }
}

fn record_pair_result(standings: &mut [ProfileStanding], a: usize, b: usize, winner: u8, a_side: u8) {
    let b_side = Board::other(a_side);
    let a_result = if winner == DRAW {
        0
    } else if winner == a_side {
        1
    } else {
        -1
    };
    let b_result = if winner == DRAW {
        0
    } else if winner == b_side {
        1
    } else {
        -1
    };
    standings[a].record(a_result);
    standings[b].record(b_result);
}

fn evaluate_tournament_generation(
    profiles: &[EvalWeights],
    games_per_pair: usize,
    depth: i32,
    time_limit: Option<Duration>,
    seed_base: u64,
) -> Vec<ProfileStanding> {
    let mut standings: Vec<_> = profiles
        .iter()
        .enumerate()
        .map(|(id, &weights)| ProfileStanding::new(id, weights))
        .collect();

    for i in 0..standings.len() {
        for j in (i + 1)..standings.len() {
            for game in 0..games_per_pair {
                let starter = if game % 2 == 0 { X } else { O };
                let seed = seed_base ^ ((i as u64) << 40) ^ ((j as u64) << 16) ^ game as u64;

                let winner_a_x = play_silent_game(
                    standings[i].weights,
                    standings[j].weights,
                    starter,
                    depth,
                    time_limit,
                    seed,
                    1 + (game % 5),
                );
                record_pair_result(&mut standings, i, j, winner_a_x, X);

                let winner_a_o = play_silent_game(
                    standings[j].weights,
                    standings[i].weights,
                    starter,
                    depth,
                    time_limit,
                    seed ^ 0xD00D_51DE_A11C_E999,
                    1 + ((game + 2) % 5),
                );
                record_pair_result(&mut standings, i, j, winner_a_o, O);
            }
        }
    }

    standings.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| a.losses.cmp(&b.losses))
    });
    standings
}

fn evolve_profiles(best: &[ProfileStanding], profile_count: usize, step_percent: i32, seed: &mut u64) -> Vec<EvalWeights> {
    let elite_count = best.len().clamp(1, 4);
    let mut profiles = Vec::with_capacity(profile_count);
    for standing in best.iter().take(elite_count) {
        profiles.push(standing.weights);
    }
    while profiles.len() < profile_count {
        let parent = best[splitmix64(seed) as usize % elite_count].weights;
        profiles.push(random_mutation(parent, seed, step_percent));
    }
    profiles
}

fn run_tournament(args: &[String]) -> io::Result<()> {
    let profile_count = args.get(2).and_then(|s| s.parse::<usize>().ok()).unwrap_or(12);
    let games_per_pair = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(2);
    let depth = args.get(4).and_then(|s| s.parse::<i32>().ok()).unwrap_or(3);
    let ms = args.get(5).and_then(|s| s.parse::<u64>().ok()).unwrap_or(100);
    let generations = args.get(6).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
    let output = args.get(7).map(String::as_str).unwrap_or("tournament.csv");
    let time_limit = if ms == 0 {
        None
    } else {
        Some(Duration::from_millis(ms))
    };

    let started = Instant::now();
    let count = profile_count.max(2);
    let mut profiles = candidate_variants(EvalWeights::default(), 14);
    profiles.truncate(count.min(profiles.len()));
    let mut rng = 0xE701_0000_u64;
    while profiles.len() < count {
        profiles.push(random_mutation(EvalWeights::default(), &mut rng, 14));
    }

    let mut standings = Vec::new();
    let mut step = 14;
    for generation in 0..generations.max(1) {
        standings = evaluate_tournament_generation(
            &profiles,
            games_per_pair,
            depth,
            time_limit,
            0x701A_0000_u64 ^ ((generation as u64) << 48),
        );
        if let Some(best) = standings.first() {
            println!(
                "Generation {}: best profile {} points={} wins={} draws={} losses={} step={}%",
                generation + 1,
                best.id,
                best.points,
                best.wins,
                best.draws,
                best.losses,
                step
            );
        }
        if generation + 1 < generations.max(1) {
            profiles = evolve_profiles(&standings, count, step, &mut rng);
            step = (step * 3 / 4).max(4);
        }
    }

    let file = File::create(output)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "rank,profile_id,points,wins,draws,losses,games,generations,macro_center,macro_corner,macro_edge,local_win,local_center,local_corner,local_edge,local_two,local_one,local_block_two,destination,mobility,closed_board_penalty"
    )?;
    for (rank, standing) in standings.iter().enumerate() {
        write!(
            writer,
            "{},{},{},{},{},{},{},{},",
            rank + 1,
            standing.id,
            standing.points,
            standing.wins,
            standing.draws,
            standing.losses,
            standing.games,
            generations.max(1)
        )?;
        write_weights_csv_columns(&mut writer, standing.weights)?;
        writeln!(writer)?;
    }
    writer.flush()?;

    if let Some(best) = standings.first() {
        println!(
            "Tournament complete: profiles={}, games_per_pair={}, depth={}, time_ms={}, generations={}, elapsed={} ms, csv={}",
            standings.len(),
            games_per_pair,
            depth,
            ms,
            generations.max(1),
            started.elapsed().as_millis(),
            output
        );
        println!(
            "Winner profile {}: points={}, wins={}, draws={}, losses={}",
            best.id, best.points, best.wins, best.draws, best.losses
        );
        print_weights(best.weights);
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--train") {
        run_training(&args);
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--bench") {
        if let Err(err) = run_benchmark(&args) {
            eprintln!("Benchmark failed: {err}");
        }
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--tournament") {
        if let Err(err) = run_tournament(&args) {
            eprintln!("Tournament failed: {err}");
        }
        return;
    }

    loop {
        let config = prompt_game_config();
        run_game(config);
        let again = prompt_choice("Play again? (`y` or `n`): ", &["y", "n"]);
        if again == "n" {
            break;
        }
    }
}

impl Searcher {
    fn new() -> Self {
        Self::with_weights(EvalWeights::default())
    }

    fn with_weights(weights: EvalWeights) -> Self {
        Self {
            weights,
            tt: vec![TtEntry::default(); TT_SIZE],
            killer_moves: [[255; 2]; MAX_PLY],
            history: [[0; 81]; 2],
            nodes: 0,
            deadline: None,
            timed_out: false,
            root_best_move: None,
            root_best_score: 0,
        }
    }

    fn clear_for_new_game(&mut self) {
        self.tt.fill(TtEntry::default());
        self.killer_moves = [[255; 2]; MAX_PLY];
        self.history = [[0; 81]; 2];
        self.nodes = 0;
        self.deadline = None;
        self.timed_out = false;
        self.root_best_move = None;
        self.root_best_score = 0;
    }

    fn search_best_move(
        &mut self,
        board: &mut Board,
        max_depth: i32,
        time_limit: Option<Duration>,
    ) -> SearchResult {
        self.nodes = 0;
        self.timed_out = false;
        self.root_best_move = None;
        self.root_best_score = 0;
        self.deadline = time_limit.map(|d| Instant::now() + d);

        let mut best_move = None;
        let mut best_score = -INF;
        let mut completed_depth = 0;
        let mut aspiration_center = 0;

        for depth in 1..=max_depth {
            let (mut alpha, mut beta) = if depth >= 4 {
                (aspiration_center - 900, aspiration_center + 900)
            } else {
                (-INF, INF)
            };

            let score = loop {
                let s = self.negamax(board, depth, alpha, beta, 0);
                if self.timed_out {
                    break s;
                }
                if s <= alpha {
                    alpha = -INF;
                    beta = min(beta + 3_000, INF);
                    continue;
                }
                if s >= beta {
                    alpha = max(alpha - 3_000, -INF);
                    beta = INF;
                    continue;
                }
                break s;
            };

            if self.timed_out {
                break;
            }

            if let Some(mv) = self.root_best_move {
                best_move = Some(mv);
                best_score = score;
                aspiration_center = score;
                completed_depth = depth;
            }
        }

        SearchResult {
            best_move,
            score: best_score,
            depth: completed_depth,
            completed: !self.timed_out,
        }
    }

    fn time_expired(&self) -> bool {
        match self.deadline {
            Some(deadline) => Instant::now() >= deadline,
            None => false,
        }
    }

    fn global_completion_targets(board: &Board, player: u8) -> [bool; 9] {
        let mut targets = [false; 9];
        for line in LINES {
            let mut mine = 0;
            let mut empty_board = None;
            let mut blocked = false;
            for &b in &line {
                match board.local_status[b] {
                    p if p == player => mine += 1,
                    EMPTY => empty_board = Some(b),
                    DRAW => blocked = true,
                    _ => blocked = true,
                }
            }
            if !blocked && mine == 2 {
                if let Some(b) = empty_board {
                    targets[b] = true;
                }
            }
        }
        targets
    }

    fn is_tactical_move(&self, board: &Board, mv: u8) -> bool {
        let idx = mv as usize;
        let player = board.current_player;
        let opp = Board::other(player);
        let local_board = Board::local_board_of(idx);
        let local_cell = Board::cell_in_local(idx);
        let bit = 1 << local_cell;
        let my_wins = board.immediate_local_wins_mask(local_board, player);
        let opp_wins = board.immediate_local_wins_mask(local_board, opp);
        (my_wins & bit) != 0 || (opp_wins & bit) != 0
    }

    fn negamax(&mut self, board: &mut Board, depth: i32, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        if (self.nodes & 2047) == 0 && self.time_expired() {
            self.timed_out = true;
            return 0;
        }
        self.nodes += 1;

        if board.global_status != EMPTY {
            return board.evaluate_for_side_to_move(&self.weights);
        }
        if depth <= 0 {
            return board.evaluate_for_side_to_move(&self.weights);
        }

        let alpha_orig = alpha;
        let tt_index = (board.hash as usize) & TT_MASK;
        let tt_entry = self.tt[tt_index];
        let mut tt_move = 255u8;

        if tt_entry.key == board.hash {
            tt_move = tt_entry.best_move;
            if tt_entry.depth as i32 >= depth {
                match tt_entry.flag {
                    0 => return tt_entry.score,
                    1 if tt_entry.score <= alpha => return tt_entry.score,
                    2 if tt_entry.score >= beta => return tt_entry.score,
                    _ => {}
                }
            }
        }

        let mut moves = [0u8; MAX_MOVES];
        let len = board.generate_moves(&mut moves);
        if len == 0 {
            return board.evaluate_for_side_to_move(&self.weights);
        }

        let ordered_len = self.order_moves(board, &mut moves, len, ply, tt_move);
        let mut best_move = moves[0];
        let mut best_score = -INF;

        for &mv in moves[..ordered_len].iter() {
            let extension = if depth == 1
                && ply < 6
                && self.is_tactical_move(board, mv)
            {
                1
            } else {
                0
            };
            let undo = board.apply_move(mv);
            let score = -self.negamax(board, depth - 1 + extension, -beta, -alpha, ply + 1);
            board.undo_move(undo);

            if self.timed_out {
                return 0;
            }

            if score > best_score {
                best_score = score;
                best_move = mv;
                if ply == 0 {
                    self.root_best_move = Some(mv);
                    self.root_best_score = score;
                }
            }

            if score > alpha {
                alpha = score;
                if alpha >= beta {
                    self.record_killer_and_history(board.current_player, mv, depth, ply);
                    break;
                }
            }
        }

        let flag = if best_score <= alpha_orig {
            1
        } else if best_score >= beta {
            2
        } else {
            0
        };
        self.store_tt(board.hash, depth, best_score, flag, best_move);
        best_score
    }

    fn record_killer_and_history(&mut self, player: u8, mv: u8, depth: i32, ply: usize) {
        if ply < MAX_PLY && self.killer_moves[ply][0] != mv {
            self.killer_moves[ply][1] = self.killer_moves[ply][0];
            self.killer_moves[ply][0] = mv;
        }
        let side_idx = (player - 1) as usize;
        self.history[side_idx][mv as usize] += depth * depth;
    }

    fn store_tt(&mut self, key: u64, depth: i32, score: i32, flag: u8, best_move: u8) {
        let idx = (key as usize) & TT_MASK;
        let replace = self.tt[idx].key != key || self.tt[idx].depth <= depth as i16;
        if replace {
            self.tt[idx] = TtEntry {
                key,
                score,
                depth: depth as i16,
                flag,
                best_move,
            };
        }
    }

    fn order_moves(
        &mut self,
        board: &mut Board,
        moves: &mut [u8; MAX_MOVES],
        len: usize,
        ply: usize,
        tt_move: u8,
    ) -> usize {
        let mut scores = [0i32; MAX_MOVES];
        let player = board.current_player;
        let opp = Board::other(player);

        let my_global_targets = Self::global_completion_targets(board, player);
        let opp_global_targets = Self::global_completion_targets(board, opp);

        for i in 0..len {
            let mv = moves[i];
            let idx = mv as usize;
            let local_board = Board::local_board_of(idx);
            let local_cell = Board::cell_in_local(idx);
            let mut score = 0i32;

            if mv == tt_move {
                score += 20_000_000;
            }
            if ply < MAX_PLY {
                if self.killer_moves[ply][0] == mv {
                    score += 1_600_000;
                } else if self.killer_moves[ply][1] == mv {
                    score += 1_300_000;
                }
            }
            score += self.history[(player - 1) as usize][idx];

            let my_wins = board.immediate_local_wins_mask(local_board, player);
            let opp_wins = board.immediate_local_wins_mask(local_board, opp);
            let completes_local_win = (my_wins & (1 << local_cell)) != 0;
            let blocks_local_win = (opp_wins & (1 << local_cell)) != 0;
            if completes_local_win {
                score += if my_global_targets[local_board] {
                    8_000_000
                } else {
                    500_000
                };
            }
            if blocks_local_win {
                score += if opp_global_targets[local_board] {
                    6_500_000
                } else {
                    420_000
                };
            }

            let undo = board.apply_move(mv);
            if board.global_status == player {
                score += 20_000_000;
            } else if board.global_status == DRAW {
                score += 8_000;
            } else {
                if undo.prev_local_status == EMPTY && board.local_status[local_board] == player {
                    score += 650_000;
                }
                if board.next_board != ANY_BOARD && board.local_status[board.next_board as usize] == EMPTY {
                    let forced = board.next_board as usize;
                    let opponent = board.current_player;
                    let opponent_winning_replies = board
                        .immediate_local_wins_mask(forced, opponent)
                        .count_ones() as i32;
                    let own_future_threats = board
                        .immediate_local_wins_mask(forced, Board::other(opponent))
                        .count_ones() as i32;
                    score -= board.local_board_feature_value(forced, board.current_player, &self.weights) * 8;
                    score += board.local_board_feature_value(
                        forced,
                        Board::other(board.current_player),
                        &self.weights,
                    ) * 6;
                    score -= board.board_mobility(forced) * 90;
                    score -= opponent_winning_replies * 220_000;
                    score += own_future_threats * 80_000;
                } else {
                    score += 25_000;
                }
            }
            board.undo_move(undo);

            score += match local_cell {
                4 => 18_000,
                0 | 2 | 6 | 8 => 9_000,
                _ => 4_000,
            };

            scores[i] = score;
        }

        for i in 0..len {
            let mut best = i;
            for j in (i + 1)..len {
                if scores[j] > scores[best] {
                    best = j;
                }
            }
            if best != i {
                scores.swap(i, best);
                moves.swap(i, best);
            }
        }
        len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(col: usize, row: usize) -> u8 {
        Board::global_to_index(col - 1, row - 1) as u8
    }

    #[test]
    fn first_move_can_be_anywhere_then_destination_is_forced() {
        let mut board = Board::new();
        assert!(board.is_legal(idx(5, 5) as usize));

        board.apply_move(idx(5, 5));

        assert_eq!(board.next_board, 4);
        assert!(board.is_legal(idx(4, 4) as usize));
        assert!(!board.is_legal(idx(1, 1) as usize));
    }

    #[test]
    fn local_win_is_detected_and_status_is_cached() {
        let mut board = Board::new();
        board.cells[Board::local_cell_index(0, 0)] = O;
        board.cells[Board::local_cell_index(0, 3)] = O;
        board.cells[Board::local_cell_index(0, 6)] = O;
        board.local_status[0] = board.detect_local_status(0);
        board.global_status = board.detect_global_status();

        assert_eq!(board.local_status[0], O);
        assert_eq!(board.global_status, EMPTY);
    }

    #[test]
    fn global_win_is_detected_from_three_local_boards() {
        let mut board = Board::new();
        board.local_status = [X, X, X, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY];
        board.global_status = board.detect_global_status();

        assert_eq!(board.global_status, X);
    }

    #[test]
    fn closed_destination_releases_next_player() {
        let mut board = Board::new();
        board.local_status[4] = X;
        board.apply_move(idx(2, 2));

        assert_eq!(board.next_board, ANY_BOARD);
    }

    #[test]
    fn apply_and_undo_restore_hash_and_rules_state() {
        let mut board = Board::new();
        let original_hash = board.hash;
        let undo = board.apply_move(idx(5, 5));
        board.undo_move(undo);

        assert_eq!(board.hash, original_hash);
        assert_eq!(board.current_player, X);
        assert_eq!(board.next_board, ANY_BOARD);
        assert_eq!(board.ply, 0);
        assert_eq!(board.cells, [EMPTY; 81]);
    }

    #[test]
    fn search_finds_immediate_global_win() {
        let mut board = Board::new();
        board.local_status = [X, X, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY];
        board.cells[Board::local_cell_index(2, 0)] = X;
        board.cells[Board::local_cell_index(2, 1)] = X;
        board.current_player = X;
        board.next_board = 2;
        board.hash = board.compute_hash();

        let mut searcher = Searcher::new();
        let result = searcher.search_best_move(&mut board, 2, Some(Duration::from_millis(200)));

        assert_eq!(result.best_move, Some(Board::local_cell_index(2, 2) as u8));
    }

    #[test]
    fn search_blocks_immediate_global_loss() {
        let mut board = Board::new();
        board.local_status = [O, O, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY];
        board.cells[Board::local_cell_index(2, 0)] = O;
        board.cells[Board::local_cell_index(2, 1)] = O;
        board.current_player = X;
        board.next_board = 2;
        board.hash = board.compute_hash();

        let mut searcher = Searcher::new();
        let result = searcher.search_best_move(&mut board, 2, Some(Duration::from_millis(200)));

        let chosen = result.best_move.expect("search should return a move");
        let undo = board.apply_move(chosen);
        let opponent_can_complete_global = board.next_board == 2
            && board.cells[Board::local_cell_index(2, 2)] == EMPTY;
        board.undo_move(undo);

        assert!(!opponent_can_complete_global);
    }

    #[test]
    fn tactical_ordering_prioritizes_macro_win_before_quiet_move() {
        let mut board = Board::new();
        board.local_status = [X, X, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY];
        board.cells[Board::local_cell_index(2, 0)] = X;
        board.cells[Board::local_cell_index(2, 1)] = X;
        board.current_player = X;
        board.next_board = 2;
        board.hash = board.compute_hash();

        let mut moves = [0u8; MAX_MOVES];
        let len = board.generate_moves(&mut moves);
        let mut searcher = Searcher::new();
        searcher.order_moves(&mut board, &mut moves, len, 0, 255);

        assert_eq!(moves[0], Board::local_cell_index(2, 2) as u8);
    }

    #[test]
    fn benchmark_stats_collect_search_metrics() {
        let stats = play_profile_game_with_stats(
            EvalWeights::default(),
            EvalWeights::default(),
            X,
            1,
            Some(Duration::from_millis(20)),
            42,
            0,
        );

        assert_ne!(stats.winner, EMPTY);
        assert!(stats.moves > 0);
        assert!(stats.searched_moves > 0);
        assert!(stats.total_nodes > 0);
    }
}
