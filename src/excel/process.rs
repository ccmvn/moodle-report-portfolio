use std::collections::HashMap;
use std::error::Error;
use chrono::{Datelike, NaiveDate, NaiveTime};
use lazy_static::lazy_static;
use log::{debug, info};
use xlsxwriter::format::{FormatAlignment, FormatBorder, FormatVerticalAlignment};
use xlsxwriter::{Format, Workbook, Worksheet};
use xlsxwriter::prelude::{GridLines, WorksheetCol, WorksheetRow};
use crate::common::tables::{Attendance, Cell, ClassbookEntry, Course};
use crate::CONFIG;

const HEALTH_REASON_ABSENCE: &str = "Keine Teilnahme am Unterricht aus gesundheitlichen Gründen";

const GRIDLINE_SETTINGS: GridLines = GridLines::HideAllGridLines;
const COLUMN_WIDTHS: [f32; 10] = [2.83, 4.33, 17.5, 2.5, 17.5, 0.55, 17.5, 0.64, 22.17, 7.33];

const FONT_NAME: &str = "Arial";
const FONT_SIZE_SMALL: f64 = 8.0;
const FONT_SIZE_LARGE: f64 = 10.0;

const FORMAT_BORDER_NONE: FormatBorder = FormatBorder::None;
const FORMAT_BORDER_MEDIUM: FormatBorder = FormatBorder::Medium;
const FORMAT_BORDER_THIN: FormatBorder = FormatBorder::Thin;

const DAY: &str = "Tag";
const OPERATIONAL_TASKS: &str = "Betriebliche Tätigkeiten, Unterweisungen, Berufsschulunterricht";
const HOURS: &str = "Stunden";

const WEEK_HOURS: &str = "Wochenstunden";
const SIGNATURE: &str = "Unterschrift:";
const TRAINING_RECORD: &str = "Ausbildungsnachweis";
const NR: &str = "Nr.";
const WEEK_FROM_TO: &str = "Woche vom bis";
const TRAINING_LOCATION: &str = "Ort der Ausbildung:";
const INSTRUCTOR: &str = "Ausbilder:";

const DAYS_AND_RANGES: &[(&str, (usize, usize, usize, usize))] = &[
    ("Montag", (3, 0, 13, 0)),
    ("Dienstag", (14, 0, 24, 0)),
    ("Mittwoch", (25, 0, 35, 0)),
    ("Donnerstag", (36, 0, 46, 0)),
    ("Freitag", (47, 0, 57, 0)),
];

lazy_static! {
    static ref WEEKDAY_TO_RANGE: HashMap<String, (usize, usize, usize, usize)> = {
        let mut m = HashMap::new();
        m.insert("Mon".to_string(), (3, 1, 13, 8));
        m.insert("Tue".to_string(), (14, 1, 24, 8));
        m.insert("Wed".to_string(), (25, 1, 35, 8));
        m.insert("Thu".to_string(), (36, 1, 46, 8));
        m.insert("Fri".to_string(), (47, 1, 57, 8));

        return m;
    };

    static ref WEEKDAY_TO_ROW: HashMap<String, usize> = {
        let mut m = HashMap::new();
        m.insert("Mon".to_string(), 13);
        m.insert("Tue".to_string(), 24);
        m.insert("Wed".to_string(), 35);
        m.insert("Thu".to_string(), 46);
        m.insert("Fri".to_string(), 57);

        return m;
    };

    static ref ZERO_HOUR_KEYWORDS: Vec<&'static str> = vec![
        "Kein Unterricht",
        "Feiertag",
        "FREI",
        "Frei",
        "Unterrichtsfrei",
        HEALTH_REASON_ABSENCE
    ];

    static ref ENTRIES: Vec<(usize, &'static str)> = vec![
        (2, "Auszubildener"),
        (4, "Ausbilder"),
        (6, "Gesetzlicher Vertreter"),
        (8, "Sonstige Sichtvermerke"),
    ];
}

// Check if the attendance is valid
pub fn is_attendance_valid(attendance: &Attendance) -> bool {
    let from_time = NaiveTime::parse_from_str(&attendance.from_time, "%H:%M")
        .map_err(|e| { info!("Failed to parse from time: {} for date {}. Error: {}", &attendance.from_time, &attendance.date, e); e })
        .ok();

    let to_time = NaiveTime::parse_from_str(&attendance.to_time, "%H:%M")
        .map_err(|e| { info!("Failed to parse to time: {} for date {}. Error: {}", &attendance.to_time, &attendance.date, e); e })
        .ok();

    let start_time = NaiveTime::from_hms_opt(0, 0, 0)
        .ok_or("Invalid start time")
        .unwrap();
    let end_time = NaiveTime::from_hms_opt(23, 59, 59)
        .ok_or("Invalid end time")
        .unwrap();

    if let (Some(from), Some(to)) = (from_time, to_time) {
        return from >= start_time && to <= end_time;
    }

    return false;
}

// Process a single course and write it to the workbook
pub async fn process_course(courses: &Vec<Course>, attendances: &Vec<Attendance>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let filename = "Reports.xlsx";
    let workbook = Workbook::new(&filename)?;

    // Initialize week number
    let mut week_number = 0;

    // Flatten and collect all entries from all courses
    let mut all_entries: Vec<_> = courses.iter().flat_map(|course| course.classbook.entries.clone()).collect();

    // Sort all entries by date and weekday
    all_entries.sort_by(|a, b| {
        let a_date = NaiveDate::parse_from_str(&a.date, "%d.%m.%y")
            .unwrap_or_else(|_| panic!("Failed to parse date: {}", &a.date));
        let b_date = NaiveDate::parse_from_str(&b.date, "%d.%m.%y")
            .unwrap_or_else(|_| panic!("Failed to parse date: {}", &b.date));

        a_date.cmp(&b_date).then_with(|| a.weekday.cmp(&b.weekday))
    });

    if all_entries.is_empty() {
        return Ok(());
    }

    let mut week_entries = vec![];
    let first_date = NaiveDate::parse_from_str(&all_entries[0].date, "%d.%m.%y")?;
    let mut last_week = first_date.iso_week().week();

    for entry in &mut all_entries {
        let entry_date = NaiveDate::parse_from_str(&entry.date, "%d.%m.%y")?;

        if let Some(attendance) = attendances.iter().find(|a| {
            let attendance_date = NaiveDate::parse_from_str(&a.date, "%d.%m.%Y");
            attendance_date == Ok(entry_date)
        }) {
            if !is_attendance_valid(attendance) {
                debug!("Invalid attendance: {} {} {}", &entry_date.format("%d.%m.%y"), &attendance.from_time, &attendance.to_time);

                entry.activities.clear();
                entry.activities.insert(HEALTH_REASON_ABSENCE.to_string());
            }
        }

        if entry_date.iso_week().week() != last_week {
            process_week(&mut week_entries, week_number, &workbook)?;
            week_number += 1;
            week_entries.clear();
        }

        week_entries.push(entry);
        last_week = entry_date.iso_week().week();
    }

    // Don't forget to process the last week
    if !week_entries.is_empty() {
        process_week(&mut week_entries, week_number, &workbook)?;
    }

    workbook.close()?;
    info!("Successfully wrote report to {}", &filename);

    Ok(())
}

// Function to set worksheet gridlines, print scale, print area and fit to pages
fn set_worksheet_gridlines(worksheet: &mut Worksheet, setting: GridLines) {
    worksheet.gridlines(setting);
    worksheet.set_print_scale(100);
    worksheet.print_across();
    worksheet.fit_to_pages(1, 1);
}

// Function to set column widths
fn set_column_widths(worksheet: &mut Worksheet, column_widths: [f32; 10]) {
    for (i, &width) in column_widths.iter().enumerate() {
        worksheet.set_column(i as WorksheetCol, i as WorksheetCol, width as f64, None).expect("Failed to set column width");
    }
}

#[derive(Clone)]
struct FormatProps {
    font_size: Option<f64>,
    font_name: &'static str,
    alignment: FormatAlignment,
    v_alignment: FormatVerticalAlignment,
    border_top: Option<FormatBorder>,
    border_bottom: Option<FormatBorder>,
    border_left: Option<FormatBorder>,
    border_right: Option<FormatBorder>,
    text_wrap: bool,
    bold: bool,
    rotation: Option<i16>,
}

impl FormatProps {
    fn new() -> Self {
        Self {
            font_size: None,
            font_name: "Arial",
            alignment: FormatAlignment::None,
            v_alignment: FormatVerticalAlignment::VerticalCenter,
            border_top: None,
            border_bottom: None,
            border_left: None,
            border_right: None,
            text_wrap: false,
            bold: false,
            rotation: None,
        }
    }
}

// Function to create a new format with the given parameters
fn create_format_from_props(format_props: FormatProps) -> Format {
    let mut format = Format::new();

    if let Some(size) = format_props.font_size {
        format.set_font_size(size);
    }

    format.set_font_name(format_props.font_name);
    format.set_align(format_props.alignment);
    format.set_vertical_align(format_props.v_alignment);

    if let Some(border) = format_props.border_top {
        format.set_border_top(border);
    }

    if let Some(border) = format_props.border_bottom {
        format.set_border_bottom(border);
    }

    if let Some(border) = format_props.border_left {
        format.set_border_left(border);
    }

    if let Some(border) = format_props.border_right {
        format.set_border_right(border);
    }

    if format_props.bold {
        format.set_bold();
    }

    if format_props.text_wrap {
        format.set_text_wrap();
    }

    if let Some(rotation) = format_props.rotation {
        format.set_rotation(rotation);
    }

    return format;
}

// Function to create a new common format
fn common_format_props(font_size: Option<f64>, border_right: Option<FormatBorder>) -> FormatProps {
    FormatProps {
        font_size,
        font_name: "Arial",
        alignment: FormatAlignment::Center,
        v_alignment: FormatVerticalAlignment::VerticalCenter,
        border_top: Some(FormatBorder::Thin),
        border_bottom: Some(FormatBorder::Thin),
        border_left: Some(FormatBorder::Thin),
        border_right,
        text_wrap: false,
        bold: false,
        rotation: None,
    }
}

// Function to create FormatProps for the cells
fn format_props_from_cell(cell: &Cell) -> FormatProps {
    let mut format_props = FormatProps::new();
    format_props.font_size = Some(cell.font_size);
    format_props.font_name = FONT_NAME;
    format_props.v_alignment = FormatVerticalAlignment::VerticalCenter;
    format_props.border_top = Some(cell.border_top);
    format_props.border_bottom = Some(FORMAT_BORDER_THIN);
    format_props.bold = cell.bold;
    format_props.border_left = if cell.start_col == 0 { Some(FORMAT_BORDER_MEDIUM) } else { Some(FORMAT_BORDER_NONE) };
    format_props.border_right = if cell.end_col == 9 { Some(FORMAT_BORDER_MEDIUM) } else { Some(FORMAT_BORDER_NONE) };

    return format_props;
}

// Function to set row heights
fn set_row_heights(worksheet: &mut Worksheet) -> Result<(), Box<dyn Error + Send + Sync>> {
    worksheet.set_row(1, 23.25, None)?;

    for row in 3..59 {
        worksheet.set_row(row, 13.0, None)?;
    }

    Ok(())
}

// Function to write blank cells to a worksheet
fn write_blank_cells(worksheet: &mut Worksheet) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut format_props = FormatProps::new();
    format_props.font_name = FONT_NAME;
    format_props.border_right = Some(FORMAT_BORDER_MEDIUM);
    format_props.v_alignment = FormatVerticalAlignment::VerticalCenter;

    let blank_cell_format = create_format_from_props(format_props);

    for row in 1..62 {
        worksheet.write_blank(row, 9, Some(&blank_cell_format))?;
    }

    Ok(())
}

// Function to write cells to a worksheet
fn write_cells(worksheet: &mut Worksheet, cells: &[Cell]) -> Result<(), Box<dyn Error + Send + Sync>> {
    for cell in cells {
        let format_props = format_props_from_cell(cell);
        let format = create_format_from_props(format_props);

        if cell.start_col == cell.end_col {
            worksheet.write_string(cell.start_row as WorksheetRow, cell.start_col as WorksheetCol, &cell.text, Some(&format))?;
        } else {
            worksheet.merge_range(cell.start_row as WorksheetRow, cell.start_col as WorksheetCol, cell.end_row as WorksheetRow, cell.end_col as WorksheetCol, &cell.text, Some(&format))?;
        }
    }

    Ok(())
}

// Function to write the header of the worksheet
fn write_day_header(worksheet: &mut Worksheet) -> Result<(), Box<dyn Error + Send + Sync>> {
    for &(day, (start_row, start_col, end_row, end_col)) in DAYS_AND_RANGES {
        let format = create_format_from_props(FormatProps {
            border_left: Some(FormatBorder::Medium),
            rotation: Some(90),
            ..common_format_props(Some(8.0), Some(FormatBorder::Thin))
        });

        worksheet.merge_range(start_row as WorksheetRow, start_col as WorksheetCol, end_row as WorksheetRow, end_col as WorksheetCol, day, Some(&format))?;
    }

    // A3
    let mut format = create_format_from_props(common_format_props(Some(10.0), Some(FormatBorder::Thin)));
    format.set_border_left(FormatBorder::Medium);

    worksheet.write_string(2, 0, DAY, Some(&format))?;

    // B3-I3
    let format = create_format_from_props(common_format_props(Some(10.0), Some(FormatBorder::Thin)));
    worksheet.merge_range(2, 1, 2, 8, OPERATIONAL_TASKS, Some(&format))?;

    // J3
    let format = create_format_from_props(common_format_props(Some(10.0), Some(FormatBorder::Medium)));
    worksheet.write_string(2, 9, HOURS, Some(&format))?;

    Ok(())
}

// HashMap for faster weekday lookup
fn entries_by_weekday<'a>(week_entries: &'a Vec<&'a ClassbookEntry>) -> HashMap<String, &'a ClassbookEntry> {
    week_entries.iter().map(|&entry| (entry.weekday.clone(), entry)).collect()
}

// Function to write the activities to the worksheet
fn write_activities(worksheet: &mut Worksheet, week_entries: &Vec<&ClassbookEntry>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let format = create_format_from_props(FormatProps {
        font_size: Some(10.0),
        font_name: "Arial",
        alignment: FormatAlignment::Left,
        v_alignment: FormatVerticalAlignment::VerticalCenter,
        border_top: Some(FormatBorder::Thin),
        border_bottom: Some(FormatBorder::Thin),
        border_left: Some(FormatBorder::Thin),
        border_right: Some(FormatBorder::Thin),
        text_wrap: true,
        bold: false,
        rotation: None,
    });

    // Create a HashMap for faster lookup
    let entries_hashmap = entries_by_weekday(week_entries);

    for (weekday, &(start_row, start_col, end_row, end_col)) in &*WEEKDAY_TO_RANGE {
        // Use the HashMap for getting the entry
        let entry = entries_hashmap.get(weekday);

        let activities_str = if let Some(entry) = entry {
            let activities_vec: Vec<String> = entry.activities.clone().into_iter().collect();
            let activities_str = activities_vec.into_iter()
                .map(|activity|
                    if activity.ends_with('?') || activity.ends_with('!') { activity + " " }
                    else { activity + ", " }
                )
                .collect::<String>();

            activities_str.trim_end_matches(", ").to_string() // Remove trailing comma and whitespace
        } else {
            "".to_string()
        };

        worksheet.merge_range(
            start_row as WorksheetRow,
            start_col as WorksheetCol,
            end_row as WorksheetRow,
            end_col as WorksheetCol,
            &activities_str,
            Some(&format),
        )?;
    }

    Ok(())
}

// Function to write the hours for each day
fn write_hours(worksheet: &mut Worksheet, week_entries: &Vec<&ClassbookEntry>) -> Result<(), Box<dyn Error + Send + Sync>> {
    let format_props = FormatProps {
        font_size: Some(10.0),
        font_name: "Arial",
        alignment: FormatAlignment::Center,
        v_alignment: FormatVerticalAlignment::None,
        border_top: None,
        border_bottom: Some(FormatBorder::Thin),
        border_left: None,
        border_right: Some(FormatBorder::Medium),
        text_wrap: false,
        bold: false,
        rotation: None,
    };
    let format = create_format_from_props(format_props);

    for (weekday, &row) in &*WEEKDAY_TO_ROW {
        let value = week_entries
            .iter()
            .find(|e| e.weekday == *weekday)
            .map_or(0f64, |entry| {
                if entry.activities.iter().any(|activity| ZERO_HOUR_KEYWORDS.iter().any(|keyword| activity.contains(keyword))) {
                    0f64
                } else {
                    8f64
                }
            });

        worksheet.write_number(row as WorksheetRow, 9, value, Some(&format))?;
    }

    Ok(())
}

// Functions to write the summary of the week
fn create_and_write(worksheet: &mut Worksheet, row: WorksheetRow, col: WorksheetCol, text: &str, props: FormatProps) -> Result<(), Box<dyn Error + Send + Sync>> {
    let format = create_format_from_props(props);
    worksheet.write_string(row, col, text, Some(&format))?;

    Ok(())
}

fn create_and_merge(worksheet: &mut Worksheet, start_row: WorksheetRow, start_col: WorksheetCol, end_row: WorksheetRow, end_col: WorksheetCol, text: &str, props: FormatProps) -> Result<(), Box<dyn Error + Send + Sync>> {
    let format = create_format_from_props(props);
    worksheet.merge_range(start_row, start_col, end_row, end_col, text, Some(&format))?;

    Ok(())
}

fn write_summary(worksheet: &mut Worksheet) -> Result<(), Box<dyn Error + Send + Sync>> {
    let base_props = FormatProps {
        font_size: None,
        font_name: FONT_NAME,
        alignment: FormatAlignment::Center,
        v_alignment: FormatVerticalAlignment::None,
        border_top: None,
        border_bottom: None,
        border_left: None,
        border_right: None,
        text_wrap: false,
        bold: false,
        rotation: None,
    };

    let border_medium_props = FormatProps { border_top: Some(FormatBorder::Medium), border_bottom: Some(FormatBorder::Medium), ..base_props.clone() };

    create_and_merge(worksheet, 58, 0, 58, 7, "", FormatProps { border_left: Some(FormatBorder::Medium), ..border_medium_props.clone() })?;
    create_and_write(worksheet, 58, 8, WEEK_HOURS, FormatProps { font_size: Some(FONT_SIZE_LARGE), border_right: Some(FormatBorder::Thin), ..border_medium_props.clone() })?;

    let format = create_format_from_props(FormatProps {
        font_size: Some(FONT_SIZE_LARGE),
        font_name: FONT_NAME,
        alignment: FormatAlignment::Center,
        v_alignment: FormatVerticalAlignment::None,
        border_top: Some(FormatBorder::Medium),
        border_bottom: Some(FormatBorder::Medium),
        border_left: Some(FormatBorder::Thin),
        border_right: Some(FormatBorder::Medium),
        text_wrap: false,
        bold: false,
        rotation: None,
    });
    worksheet.write_formula_num(58, 9, "=SUM(J14:J58)", Some(&format), 30.)?;

    let border_medium_center_props = FormatProps { v_alignment: FormatVerticalAlignment::VerticalCenter, border_top: Some(FormatBorder::Medium), border_bottom: Some(FormatBorder::Medium), border_left: Some(FormatBorder::Medium), ..base_props.clone() };
    create_and_merge(worksheet, 59, 0, 62, 1, SIGNATURE, FormatProps { font_size: Some(FONT_SIZE_SMALL), ..border_medium_center_props.clone() })?;

    create_and_write(worksheet, 60, 2, CONFIG.get_signature(), FormatProps { font_name: CONFIG.get_font_name(), font_size: Some(CONFIG.get_font_size() as f64), border_bottom: Some(FormatBorder::Thin), ..base_props.clone() })?;

    for &(col, text) in &*ENTRIES {
        create_and_write(worksheet, 61, col as WorksheetCol, text, FormatProps { font_size: Some(FONT_SIZE_SMALL), v_alignment: FormatVerticalAlignment::VerticalBottom, border_top: Some(FormatBorder::Thin), ..base_props.clone() })?;
    }

    let border_bottom_medium_props = FormatProps { border_bottom: Some(FormatBorder::Medium), ..base_props.clone() };
    create_and_merge(worksheet, 62, 2, 62, 8, "", FormatProps { font_size: Some(FONT_SIZE_SMALL), v_alignment: FormatVerticalAlignment::VerticalBottom, ..border_bottom_medium_props.clone() })?;
    create_and_write(worksheet, 62, 9, "", FormatProps { border_right: Some(FormatBorder::Medium), ..border_bottom_medium_props })?;

    Ok(())
}

// Process a week of entries and write them to the workbook
fn process_week(week_entries: &Vec<&ClassbookEntry>, week_number: u32, workbook: &Workbook) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (start_date, end_date) = (
        &week_entries.first().unwrap().date,
        &week_entries.last().unwrap().date,
    );
    let date_range = format!("{} - {}", start_date, end_date);
    let mut worksheet = workbook.add_worksheet(Some(&format!("{}", &date_range)))?;

    set_worksheet_gridlines(&mut worksheet, GRIDLINE_SETTINGS);
    set_column_widths(&mut worksheet, COLUMN_WIDTHS);

    set_row_heights(&mut worksheet).expect("Failed to set row heights");
    write_blank_cells(&mut worksheet)?;

    let cells = [
        Cell {
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 2,
            text: TRAINING_RECORD.to_string(),
            font_size: 13.0,
            bold: true,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 3,
            end_row: 0,
            end_col: 3,
            text: NR.to_string(),
            font_size: 11.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 4,
            end_row: 0,
            end_col: 4,
            text: (week_number + 1).to_string(),
            font_size: 10.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 5,
            end_row: 0,
            end_col: 5,
            text: "".to_string(),
            font_size: 10.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 6,
            end_row: 0,
            end_col: 6,
            text: WEEK_FROM_TO.to_string(),
            font_size: 11.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 7,
            end_row: 0,
            end_col: 7,
            text: "".to_string(),
            font_size: 10.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 8,
            end_row: 0,
            end_col: 8,
            text: date_range.clone(),
            font_size: 10.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        },
        Cell {
            start_row: 0,
            start_col: 9,
            end_row: 0,
            end_col: 9,
            text: "".to_string(),
            font_size: 10.0,
            bold: false,
            border_top: FORMAT_BORDER_MEDIUM
        }
    ];

    write_cells(&mut worksheet, &cells)?;

    let format = create_format_from_props(FormatProps {
        font_size: Some(10.0),
        font_name: "Arial",
        alignment: FormatAlignment::None,
        v_alignment: FormatVerticalAlignment::VerticalCenter,
        border_top: None,
        border_bottom: None,
        border_left: None,
        border_right: None,
        text_wrap: false,
        bold: false,
        rotation: None,
    });
    worksheet.write_string(1, 4, CONFIG.get_location(), Some(&format))?;
    worksheet.write_string(1, 6, INSTRUCTOR, Some(&format))?;
    worksheet.write_string(1, 8, CONFIG.get_educator_name(), Some(&format))?;

    let format = create_format_from_props(FormatProps {
        font_size: Some(11.0),
        font_name: "Arial",
        alignment: FormatAlignment::None,
        v_alignment: FormatVerticalAlignment::VerticalCenter,
        border_top: None,
        border_bottom: None,
        border_left: Some(FormatBorder::Medium),
        border_right: None,
        text_wrap: false,
        bold: false,
        rotation: None,
    });
    worksheet.merge_range(1, 0, 1, 3, TRAINING_LOCATION, Some(&format))?;

    write_day_header(&mut worksheet)?;
    write_activities(&mut worksheet, &week_entries)?;
    write_hours(&mut worksheet, &week_entries)?;
    write_summary(&mut worksheet)?;

    Ok(())
}
