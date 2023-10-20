use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::thread_rng;
use std::{fmt::Display, io::stdin};

macro_rules! unwrap_or_continue {
    ($fallible:expr) => {
        match $fallible {
            Ok(success) => success,
            Err(e) => {
                println!("{e}");
                continue;
            }
        }
    };
}

#[derive(Debug, Clone)]
enum CellState {
    Bomb { flagged: bool },
    Safe { flagged: bool, open: bool },
}

// WARNING: there are no checks to ensure this has valid indeces;
// it is only intended as a convenient abstraction
#[derive(Debug, Clone, Copy)]
struct CellPosition {
    row_index: usize,
    col_index: usize,
}

impl PartialEq for CellPosition {
    fn eq(&self, other: &Self) -> bool {
        self.row_index == other.row_index && self.col_index == other.col_index
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

#[derive(Debug, Clone)]
struct Cell {
    bombs_around: u8,
    state: CellState,
    position: CellPosition,
}

impl Cell {
    fn new<const N: usize>(
        row_index: usize,
        col_index: usize,
        bombs: &[[bool; N]; N],
        is_bomb: bool,
    ) -> Self {
        let position = CellPosition {
            row_index,
            col_index,
        };

        Cell {
            bombs_around: get_bombs_around(bombs, position),
            state: if is_bomb {
                CellState::Bomb { flagged: false }
            } else {
                CellState::Safe {
                    flagged: false,
                    open: false,
                }
            },
            position,
        }
    }
}

impl Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let out = match self.state {
            CellState::Bomb { flagged: true } => "^",
            CellState::Bomb { flagged: false } => "@",
            CellState::Safe {
                flagged: true,
                open: true,
            } => panic!("Cell should not be both flagged and open"),
            CellState::Safe {
                flagged: true,
                open: false,
            } => "?",
            CellState::Safe {
                flagged: false,
                open: true,
            } => return write!(f, "{}", self.bombs_around),
            CellState::Safe {
                flagged: false,
                open: false,
            } => "#",
        };

        write!(f, "{out}")
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

#[derive(Debug)]
enum ClearError {
    ClearedBomb,
    CellNotFound,
    AlreadyCleared,
}

#[derive(Clone)]
struct Board<const N: usize> {
    board: [[Cell; N]; N],
}

impl<const N: usize> Board<N> {
    fn new() -> Self {
        let bombs: [[bool; N]; N] = generate_bombs();

        let cells: [[Cell; N]; N] = bombs
            .iter()
            .enumerate()
            .map(|(row_index, row)| {
                row.iter()
                    .enumerate()
                    .map(|(col_index, &is_bomb)| Cell::new(row_index, col_index, &bombs, is_bomb))
                    .collect::<Vec<Cell>>()
                    .try_into()
                    .expect("Vector of cells should have the correct length")
            })
            .collect::<Vec<[Cell; N]>>()
            .try_into()
            .expect("Vector of vector of cells should have the correct length");

        Board { board: cells }
    }

    fn get_cell_mut(&mut self, position: CellPosition) -> Option<&mut Cell> {
        self.board
            .get_mut(position.row_index)
            .and_then(|row| row.get_mut(position.col_index))
    }

    fn is_won(&self) -> bool {
        let cells = self.board.concat();
        // check if there is any cell that is closed and safe
        !cells.iter().any(|cell| match cell.state {
            CellState::Safe { open: false, .. } => true,
            _ => false,
        })
    }

    fn clear(
        &mut self,
        position: CellPosition,
        traversed: &Vec<CellPosition>,
    ) -> Result<(), ClearError> {
        let board_before_mutation = self.board.clone();

        if traversed.contains(&position) {
            return Ok(());
        }

        let cell = self
            .get_cell_mut(position)
            .ok_or(ClearError::CellNotFound)?;

        match cell.state {
            CellState::Bomb { .. } => return Err(ClearError::ClearedBomb),
            CellState::Safe { open: true, .. } => return Err(ClearError::AlreadyCleared),
            CellState::Safe { open: false, .. } => {
                cell.state = CellState::Safe {
                    open: true,
                    flagged: false,
                }
            }
        };

        if cell.bombs_around == 0 {
            let new_traversed = [&traversed[..], &[cell.position]].concat();

            for cell_around in get_cells_around(&board_before_mutation, position) {
                self.clear(cell_around.position, &new_traversed)
                    .unwrap_or_else(|err| match err {
                        ClearError::CellNotFound => {
                            panic!("get_cells_around should return only valid cells")
                        }
                        ClearError::ClearedBomb => {
                            panic!("Cell with bombs_around==0 should have no bombs around it")
                        }
                        ClearError::AlreadyCleared => (),
                    });
            }
        }

        Ok(())
    }
}

impl<const N: usize> Display for Board<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let out = self
            .board
            .clone()
            .map(|row| row.map(|cell| cell.to_string()).join(" "))
            .join("\n");
        write!(f, "{out}")
    }
}

fn get_cells_around<T, const N: usize>(
    board: &[[T; N]; N],
    position: CellPosition,
) -> impl Iterator<Item = &T> {
    let CellPosition {
        row_index,
        col_index,
    } = position;

    let min_row_index = if row_index > 0 { row_index - 1 } else { 0 };
    let max_row_index = if row_index < N - 1 {
        row_index + 1
    } else {
        N - 1
    };

    let min_col_index = if col_index > 0 { col_index - 1 } else { 0 };
    let max_col_index = if col_index < N - 1 {
        col_index + 1
    } else {
        N - 1
    };

    board
        .get(min_row_index..max_row_index + 1)
        .expect("Hardcoded row bounds checks should succeed")
        .iter()
        .map(move |row| {
            row.get(min_col_index..max_col_index + 1)
                .expect("Hardcoded col bounds checks should succeed")
        })
        .flatten()
}

fn get_bombs_around<const N: usize>(board: &[[bool; N]; N], position: CellPosition) -> u8 {
    let cells_around = get_cells_around(board, position);
    let num_bombs_around = cells_around.filter(|&&is_bomb| is_bomb).count();
    num_bombs_around as u8
}

fn generate_bombs<const N: usize>() -> [[bool; N]; N] {
    let mut rng = thread_rng();

    // true = bomb; false = safe
    let choices = [true, false];
    // 1 bomb for every 5 safe tiles
    // (16.66% bombs)
    let weights = [1, 5];
    let dist = WeightedIndex::new(&weights).expect("Hardcoded weights are correct");

    (0..N)
        .map(|_| {
            (0..N)
                .map(|_| choices[dist.sample(&mut rng)])
                .collect::<Vec<bool>>()
                .try_into()
                .expect("Vector of booleans should have the correct length")
        })
        .collect::<Vec<[bool; N]>>()
        .try_into()
        .expect("Vector of vectors of booleans should have the correct length")
}

fn main() {
    let mut board = Board::<10>::new();

    let result = loop {
        if board.is_won() {
            break "Game won";
        }

        println!("{board}\n------");

        let mut row_index = String::new();
        println!("Enter row index:");

        unwrap_or_continue!(stdin().read_line(&mut row_index));
        let row_index: usize = unwrap_or_continue!(row_index.trim_end().parse());

        let mut col_index = String::new();
        println!("Enter col index:");

        unwrap_or_continue!(stdin().read_line(&mut col_index));
        let col_index: usize = unwrap_or_continue!(col_index.trim_end().parse());

        let position = CellPosition {
            row_index,
            col_index,
        };

        match board.clear(position, &vec![]) {
            Ok(_) => (),
            Err(ClearError::CellNotFound) => {
                println!("Invalid cell position");
                continue;
            }
            Err(ClearError::ClearedBomb) => {
                break "Game lost";
            }
            Err(ClearError::AlreadyCleared) => {
                println!("Cell already cleared");
                continue;
            }
        };

        println!("------");
    };

    println!("{result}")
}
