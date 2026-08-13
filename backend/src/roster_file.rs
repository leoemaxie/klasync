use std::io::Cursor;

use calamine::{Reader, Xlsx};

use crate::models::RosterStudent;

pub struct ParsedRoster {
    pub students: Vec<RosterStudent>,
    pub issues: Vec<String>,
}

pub fn parse(file_name: &str, bytes: &[u8]) -> Result<ParsedRoster, String> {
    if file_name.to_ascii_lowercase().ends_with(".csv") {
        parse_csv(bytes)
    } else if file_name.to_ascii_lowercase().ends_with(".xlsx") {
        parse_xlsx(bytes)
    } else {
        Err("Unsupported file format".to_owned())
    }
}

pub async fn parse_async(file_name: String, bytes: Vec<u8>) -> Result<ParsedRoster, String> {
    tokio::task::spawn_blocking(move || parse(&file_name, &bytes))
        .await
        .map_err(|join_err| {
            tracing::error!(%join_err, "Roster parsing task panicked");
            "Internal server error during roster parsing".to_owned()
        })?
}


fn parse_csv(bytes: &[u8]) -> Result<ParsedRoster, String> {
    let mut reader = csv::ReaderBuilder::new().flexible(true).from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|_| "Invalid CSV headers".to_owned())?
        .clone();
    let mapping = ColumnMapping::from_headers(headers.iter())?;
    let rows = reader.records().enumerate().map(|(index, row)| {
        let row = row.map_err(|_| format!("Row_{}: invalid csv", index + 2))?;
        Ok((index + 2, mapping.read_csv(&row)))
    });
    collect_rows(rows)
}

fn parse_xlsx(bytes: &[u8]) -> Result<ParsedRoster, String> {
    let mut workbook =
        Xlsx::new(Cursor::new(bytes)).map_err(|_| "Invalid XLSX workbook".to_owned())?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| "Missing XLSX sheet".to_owned())
        .and_then(|result| result.map_err(|_| "Invalid XLSX sheet".to_owned()))?;
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or_else(|| "Missing XLSX headers".to_owned())?;
    let mapping = ColumnMapping::from_headers(headers.iter().map(ToString::to_string))?;
    collect_rows(rows.enumerate().map(|(index, row)| {
        let values: Vec<String> = row.iter().map(ToString::to_string).collect();
        Ok((index + 2, mapping.read_values(&values)))
    }))
}

fn collect_rows<I>(rows: I) -> Result<ParsedRoster, String>
where
    I: IntoIterator<Item = Result<(usize, (String, String, Option<String>)), String>>,
{
    let mut students = Vec::new();
    let mut issues = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for row in rows {
        let (row_number, (matric_number, full_name, email)) = row?;
        if matric_number.is_empty() && full_name.is_empty() {
            continue;
        }
        if matric_number.is_empty() || full_name.is_empty() {
            issues.push(format!("Row_{row_number}: missing matric or name"));
            continue;
        }
        if !seen.insert(matric_number.to_ascii_lowercase()) {
            issues.push(format!("Row_{row_number}: duplicate matric number"));
            continue;
        }
        students.push(RosterStudent {
            matric_number,
            full_name,
            email,
        });
    }
    Ok(ParsedRoster { students, issues })
}

struct ColumnMapping {
    matric: usize,
    name: usize,
    email: Option<usize>,
}

impl ColumnMapping {
    fn from_headers<I>(headers: I) -> Result<Self, String>
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let headers: Vec<String> = headers
            .into_iter()
            .map(|header| normalize(&header.to_string()))
            .collect();
        let find = |names: &[&str]| {
            headers
                .iter()
                .position(|header| names.contains(&header.as_str()))
        };
        Ok(Self {
            matric: find(&["matric_number", "matric", "student_id", "student_number"])
                .ok_or_else(|| "Missing matric column".to_owned())?,
            name: find(&["full_name", "name", "student_name"])
                .ok_or_else(|| "Missing name column".to_owned())?,
            email: find(&["email", "email_address"]),
        })
    }

    fn read_csv(&self, row: &csv::StringRecord) -> (String, String, Option<String>) {
        self.read_values(&row.iter().map(ToOwned::to_owned).collect::<Vec<_>>())
    }

    fn read_values(&self, values: &[String]) -> (String, String, Option<String>) {
        let value = |index| {
            values
                .get(index)
                .map(|value: &String| value.trim().to_owned())
                .unwrap_or_default()
        };
        let email = self.email.map(value).filter(|email| !email.is_empty());
        (value(self.matric), value(self.name), email)
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}
