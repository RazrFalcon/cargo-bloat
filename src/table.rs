use std::{cmp, fmt};

pub struct Table {
    data: Vec<String>,
    columns_count: usize,
    max_width: Option<usize>,
}

impl Table {
    pub fn new(header: &[&str]) -> Self {
        assert!(!header.is_empty());

        Table {
            data: header.iter().map(|s| s.to_string()).collect(),
            columns_count: header.len(),
            max_width: None,
        }
    }

    pub fn set_width(&mut self, w: Option<usize>) {
        self.max_width = w;
    }

    pub fn push(&mut self, row: &[String]) {
        assert_eq!(row.len(), self.columns_count);
        self.data.extend_from_slice(row);
    }

    pub fn rows_count(&self) -> usize {
        debug_assert!(!self.data.is_empty());

        self.data.len() / self.columns_count - 1
    }

    pub fn cell(&self, row: usize, col: usize) -> &str {
        &self.data[row * self.columns_count + col]
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rows = self.rows_count() + 1; // plus header

        let mut max_len_list = Vec::new();
        for col in 0..self.columns_count {
            let mut max = 0;
            for row in 0..rows {
                max = cmp::max(max, self.cell(row, col).len());
            }
            max_len_list.push(max);
        }

        // Width of all columns except the last one.
        let non_fn_width: usize = max_len_list.iter().take(max_len_list.len() - 1).sum();
        // Count spaces between columns.
        let non_fn_width = non_fn_width + self.columns_count - 1;

        let mut row = 0;
        while row < rows {
            let row_data = &self.data[row * self.columns_count .. (row + 1) * self.columns_count];

            for (col, cell) in row_data.iter().enumerate() {
                if col != self.columns_count - 1 {
                    let pad = max_len_list[col] - cell.len();
                    for _ in 0..pad {
                        write!(f, " ")?;
                    }

                    write!(f, "{} ", cell)?;
                } else {
                    if let Some(w) = self.max_width {
                        let name_width = w - non_fn_width;

                        let mut cell = cell.clone();
                        if cell.len() > name_width {
                            cell.drain((name_width - 3)..);
                            cell.push_str("...");
                        }
                        writeln!(f, "{}", cell)?;
                    } else {
                        writeln!(f, "{}", cell)?;
                    }
                }
            }

            row += 1;
        }

        Ok(())
    }
}
