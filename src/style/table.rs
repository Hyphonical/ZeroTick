use super::SUBTLE;
use colored::Colorize;

/// A simple rounded-corner Unicode box table renderer.
pub struct Table {
	headers: Vec<String>,
	rows: Vec<Vec<String>>,
}

impl Table {
	pub fn new(headers: Vec<impl Into<String>>) -> Self {
		Self {
			headers: headers.into_iter().map(Into::into).collect(),
			rows: Vec::new(),
		}
	}

	pub fn add_row(&mut self, row: Vec<impl Into<String>>) {
		self.rows.push(row.into_iter().map(Into::into).collect());
	}

	pub fn render(&self) -> String {
		let cols = self.headers.len();
		let mut widths = vec![0usize; cols];

		// Measure column widths
		for (i, h) in self.headers.iter().enumerate() {
			widths[i] = widths[i].max(visible_len(h));
		}
		for row in &self.rows {
			for (i, cell) in row.iter().enumerate() {
				if i < cols {
					widths[i] = widths[i].max(visible_len(cell));
				}
			}
		}

		// Add padding
		for w in &mut widths {
			*w += 2; // 1 space each side
		}

		let mut out = String::new();

		// Top border: ╭──┬──╮
		out.push_str(&self.border_line("╭", "┬", "╮", &widths));

		// Header row
		out.push_str(&self.data_line(&self.headers, &widths, true));

		// Header separator: ├──┼──┤
		out.push_str(&self.border_line("├", "┼", "┤", &widths));

		// Data rows
		for row in &self.rows {
			out.push_str(&self.data_line(row, &widths, false));
		}

		// Bottom border: ╰──┴──╯
		out.push_str(&self.border_line("╰", "┴", "╯", &widths));

		out
	}

	fn border_line(&self, left: &str, mid: &str, right: &str, widths: &[usize]) -> String {
		let mut s = String::new();
		s.push_str(&left.color(SUBTLE).to_string());
		for (i, &w) in widths.iter().enumerate() {
			s.push_str(&"─".repeat(w).color(SUBTLE).to_string());
			if i < widths.len() - 1 {
				s.push_str(&mid.color(SUBTLE).to_string());
			}
		}
		s.push_str(&right.color(SUBTLE).to_string());
		s.push('\n');
		s
	}

	fn data_line(&self, cells: &[String], widths: &[usize], is_header: bool) -> String {
		let mut s = String::new();
		s.push_str(&"│".color(SUBTLE).to_string());
		for (i, w) in widths.iter().enumerate() {
			let text = cells.get(i).map(|s| s.as_str()).unwrap_or("");
			let vis_len = visible_len(text);
			let padding = if *w > vis_len + 1 { w - vis_len - 1 } else { 0 };

			s.push(' ');
			if is_header {
				s.push_str(&text.white().bold().to_string());
			} else {
				s.push_str(text);
			}
			s.push_str(&" ".repeat(padding));
			s.push_str(&"│".color(SUBTLE).to_string());
		}
		s.push('\n');
		s
	}
}

/// Estimate visible length ignoring ANSI escape codes.
fn visible_len(s: &str) -> usize {
	let mut len = 0;
	let mut in_escape = false;
	for ch in s.chars() {
		if in_escape {
			if ch.is_ascii_alphabetic() {
				in_escape = false;
			}
		} else if ch == '\x1b' {
			in_escape = true;
		} else {
			len += 1;
		}
	}
	len
}
