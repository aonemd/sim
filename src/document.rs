use crate::Position;
use crate::Row;
use crate::SearchDirection;
use crate::FileType;
use std::fs;
use std::io::{Error, Write};

#[derive(Default)]
pub struct Document {
    rows: Vec<Row>,
    pub file_name: Option<String>,
    dirty: bool,
    file_type: FileType,
}

impl Document {
    /// # Errors
    ///
    pub fn open(filename: &str) -> Result<Self, std::io::Error> {
        let contents = fs::read_to_string(filename)?;
        let file_type = FileType::from(filename);

        let mut rows = Vec::new();
        for line in contents.lines() {
            let mut row = Row::from(line);
            row.highlight(&file_type.highlighting_options(), None, false);
            rows.push(row);
        }

        Ok(Self {
            rows,
            file_name: Some(filename.to_string()),
            dirty: false,
            file_type,
        })
    }

    pub fn file_type(&self) -> String {
        self.file_type.name()
    }

    #[must_use]
    pub fn row(&self, index: usize) -> Option<&Row> {
        self.rows.get(index)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn insert(&mut self, at: &Position, c: char) {
        if at.y > self.len() {
            return;
        }

        self.dirty = true;

        if c == '\n' {
            self.insert_newline(at);
            return;
        }

        if at.y == self.len() {
            let mut row = Row::default();
            row.insert(0, c);
            row.highlight(&self.file_type.highlighting_options(), None, false);
            self.rows.push(row);
        } else {
            #[allow(clippy::indexing_slicing)]
            let row = &mut self.rows[at.y];
            row.insert(at.x, c);
            row.highlight(&self.file_type.highlighting_options(), None, false);
        }
    }

    fn insert_newline(&mut self, at: &Position) {
        if at.y > self.len() {
            return;
        }

        if at.y == self.len() {
            self.rows.push(Row::default());
            return;
        }

        #[allow(clippy::indexing_slicing)]
        let current_row = &mut self.rows[at.y];
        let mut new_row = current_row.split(at.x);
        current_row.highlight(&self.file_type.highlighting_options(), None, false);
        new_row.highlight(&self.file_type.highlighting_options(), None, false);

        #[allow(clippy::integer_arithmetic)]
        self.rows.insert(at.y + 1, new_row);
    }

    #[allow(clippy::integer_arithmetic, clippy::indexing_slicing)]
    pub fn delete(&mut self, at: &Position) {
        let len = self.len();
        if at.y >= len {
            return;
        }

        self.dirty = true;

        // checking if we are at the end of a line, and if a line follows after this one
        if at.x == self.rows[at.y].len() && at.y + 1< len {
            // If that’s the case, we remove the next line from our vec and append it to the
            // current row
            //
            // we can’t have a reference to row and then delete part of the vector. So we first
            // read row’s length directly without retaining a reference. Then we mutate the vector
            // by removing an element from it, and then we create our mutable reference to row
            let next_row = self.rows.remove(at.y + 1);
            let row = &mut self.rows[at.y];
            row.append(&next_row);
            row.highlight(&self.file_type.highlighting_options(), None, false);
        } else {
            //  If that’s not the case, we simply try to delete from the current row
            let row = &mut self.rows[at.y];
            row.delete(at.x);
            row.highlight(&self.file_type.highlighting_options(), None, false);
        }
    }

    /// # Errors
    ///
    pub fn save(&mut self) -> Result<(), Error> {
        if let Some(file_name) = &self.file_name {
            let mut file = fs::File::create(file_name)?;
            self.file_type = FileType::from(file_name);
            let mut starts_with_comment = false;
            for row in &mut self.rows {
                file.write_all(row.as_bytes())?;
                file.write_all(b"\n")?;
                starts_with_comment = row.highlight(&self.file_type.highlighting_options(), None, starts_with_comment)
            }

            self.dirty = false;
        }

        Ok(())
    }

    pub fn highlight(&mut self, word: Option<&str>) {
        let mut starts_with_comment = false;
        for row in &mut self.rows {
            starts_with_comment = row.highlight(&self.file_type.highlighting_options(), word, starts_with_comment)
        }
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    #[allow(clippy::indexing_slicing)]
    pub fn find(&self, query: &str, at: &Position, direction: SearchDirection) -> Option<Position> {
        if at.y >= self.len() {
            return None;
        }

        let mut position = Position { x: at.x, y: at.y };

        let start = if direction == SearchDirection::Forward {
            at.y
        } else {
            0
        };
        let end = if direction == SearchDirection::Forward {
            self.rows.len()
        } else {
            at.y.saturating_add(1)
        };
        for _ in start..end {
            if let Some(row) = self.rows.get(position.y) {
                if let Some(x) = row.find(&query, position.x, direction) {
                    position.x = x;
                    return Some(position);
                }
                if direction == SearchDirection::Forward {
                    position.y = position.y.saturating_add(1);
                    position.x = 0;
                } else {
                    position.y = position.y.saturating_sub(1);
                    position.x = self.rows[position.y].len();
                }
            } else {
                return None;
            }
        }

        None
    }
}
