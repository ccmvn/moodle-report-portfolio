use linked_hash_set::LinkedHashSet;
use serde_derive::Serialize;
use xlsxwriter::format::FormatBorder;

#[derive(Serialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
    pub logintoken: String,
}

#[derive(Clone)]
pub struct Classbook {
    pub id: String,
    pub link: String,
    pub direct_link: Option<String>,
    pub entries: Vec<ClassbookEntry>,
}

#[derive(Clone)]
pub struct ClassbookEntry {
    pub weekday: String,
    pub date: String,
    pub time: String,
    pub description: String,
    pub activities: LinkedHashSet<String>,
}

#[derive(Clone)]
pub struct Course {
    pub id: String,
    pub name: String,
    pub link: String,
    pub course: String,
    pub duration: String,
    pub classbook: Classbook,
}

#[derive(Clone)]
pub struct Attendance {
    pub date: String,
    pub from_time: String,
    pub to_time: String,
}

#[derive(Clone)]
pub struct Cell {
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
    pub text: String,
    pub font_size: f64,
    pub bold: bool,
    pub border_top: FormatBorder,
}
