use super::SUBTLE;
use colored::Colorize;

const BRANCH: &str = "├─";
const LAST: &str = "╰─";
const PIPE: &str = "│ ";
const BLANK: &str = "  ";

/// Builds a styled tree output, handling indentation and branch characters.
pub struct Tree {
	lines: Vec<TreeLine>,
}

struct TreeLine {
	label: String,
	value: String,
	children: Vec<TreeLine>,
}

impl Tree {
	pub fn new() -> Self {
		Self { lines: Vec::new() }
	}

	pub fn add(&mut self, label: impl Into<String>, value: impl Into<String>) {
		self.lines.push(TreeLine {
			label: label.into(),
			value: value.into(),
			children: Vec::new(),
		});
	}

	pub fn add_with_children(
		&mut self,
		label: impl Into<String>,
		value: impl Into<String>,
		children: Vec<(String, String)>,
	) {
		self.lines.push(TreeLine {
			label: label.into(),
			value: value.into(),
			children: children
				.into_iter()
				.map(|(l, v)| TreeLine {
					label: l,
					value: v,
					children: Vec::new(),
				})
				.collect(),
		});
	}

	/// Render the tree to a string with ANSI styling.
	pub fn render(&self, indent: usize) -> String {
		let mut out = String::new();
		let total = self.lines.len();

		for (i, line) in self.lines.iter().enumerate() {
			let is_last = i == total - 1;
			let prefix = if is_last { LAST } else { BRANCH };
			let continuation = if is_last { BLANK } else { PIPE };

			// Pad labels to align values.
			let _pad = " ".repeat(indent);
			out.push_str(&format!(
				" {} {} {}\n",
				prefix.color(SUBTLE),
				line.label.color(super::MUTED),
				line.value,
			));

			for (j, child) in line.children.iter().enumerate() {
				let child_last = j == line.children.len() - 1;
				let child_prefix = if child_last { LAST } else { BRANCH };
				out.push_str(&format!(
					" {}  {} {} {}\n",
					continuation.color(SUBTLE),
					child_prefix.color(SUBTLE),
					child.label.color(super::MUTED),
					child.value,
				));
			}
		}

		out
	}
}

impl Default for Tree {
	fn default() -> Self {
		Self::new()
	}
}
