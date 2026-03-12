use crate::types::{ChatComponent, Description};
use colored::{Color, ColoredString, Colorize};

/// Render a `Description` (MOTD) into ANSI-colored text.
pub fn render(description: &Description) -> String {
	match description {
		Description::Plain(s) => render_legacy_codes(s),
		Description::Component(component) => render_component(component),
	}
}

/// Render a `ChatComponent` tree into styled text.
fn render_component(comp: &ChatComponent) -> String {
	let mut out = String::new();

	let text = &comp.text;
	if !text.is_empty() {
		let styled = apply_style(text, comp);
		out.push_str(&styled);
	}

	if let Some(ref translate) = comp.translate {
		let styled = apply_style(translate, comp);
		out.push_str(&styled);
	}

	for child in &comp.extra {
		out.push_str(&render_component(child));
	}

	out
}

/// Apply color and formatting from a ChatComponent to a string.
fn apply_style(text: &str, comp: &ChatComponent) -> String {
	// First handle any legacy § codes in the text itself
	let base = render_legacy_codes(text);

	// If the component has explicit styling, apply it
	if comp.color.is_none()
		&& comp.bold.is_none()
		&& comp.italic.is_none()
		&& comp.underlined.is_none()
		&& comp.strikethrough.is_none()
	{
		return base;
	}

	let mut styled: ColoredString = base.normal();

	if let Some(ref color) = comp.color {
		if let Some(c) = mc_color_to_colored(color) {
			styled = styled.color(c);
		}
	}

	if comp.bold == Some(true) {
		styled = styled.bold();
	}
	if comp.italic == Some(true) {
		styled = styled.italic();
	}
	if comp.underlined == Some(true) {
		styled = styled.underline();
	}
	if comp.strikethrough == Some(true) {
		styled = styled.strikethrough();
	}
	if comp.obfuscated == Some(true) {
		styled = styled.dimmed();
	}

	styled.to_string()
}

/// Render legacy `§` formatting codes into ANSI.
pub fn render_legacy_codes(text: &str) -> String {
	let mut out = String::new();
	let mut chars = text.chars().peekable();
	let mut current_color: Option<Color> = None;
	let mut bold = false;
	let mut italic = false;
	let mut underline = false;
	let mut strike = false;

	while let Some(ch) = chars.next() {
		if ch == '\u{00A7}' {
			if let Some(&code) = chars.peek() {
				chars.next();
				match code {
					'0' => {
						current_color = Some(Color::Black);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'1' => {
						current_color = Some(Color::Blue);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'2' => {
						current_color = Some(Color::Green);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'3' => {
						current_color = Some(Color::Cyan);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'4' => {
						current_color = Some(Color::Red);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'5' => {
						current_color = Some(Color::Magenta);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'6' => {
						current_color = Some(Color::TrueColor {
							r: 255,
							g: 170,
							b: 0,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'7' => {
						current_color = Some(Color::TrueColor {
							r: 170,
							g: 170,
							b: 170,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'8' => {
						current_color = Some(Color::TrueColor {
							r: 85,
							g: 85,
							b: 85,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'9' => {
						current_color = Some(Color::TrueColor {
							r: 85,
							g: 85,
							b: 255,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'a' | 'A' => {
						current_color = Some(Color::TrueColor {
							r: 85,
							g: 255,
							b: 85,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'b' | 'B' => {
						current_color = Some(Color::TrueColor {
							r: 85,
							g: 255,
							b: 255,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'c' | 'C' => {
						current_color = Some(Color::TrueColor {
							r: 255,
							g: 85,
							b: 85,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'd' | 'D' => {
						current_color = Some(Color::TrueColor {
							r: 255,
							g: 85,
							b: 255,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'e' | 'E' => {
						current_color = Some(Color::TrueColor {
							r: 255,
							g: 255,
							b: 85,
						});
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'f' | 'F' => {
						current_color = Some(Color::White);
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					'l' | 'L' => bold = true,
					'o' | 'O' => italic = true,
					'n' | 'N' => underline = true,
					'm' | 'M' => strike = true,
					'k' | 'K' => {} // obfuscated — can't do in terminal
					'r' | 'R' => {
						current_color = None;
						bold = false;
						italic = false;
						underline = false;
						strike = false;
					}
					_ => {
						out.push('§');
						out.push(code);
					}
				}
			}
		} else {
			// Apply current formatting to this character
			let s = ch.to_string();
			let mut styled: ColoredString = if let Some(c) = current_color {
				s.color(c)
			} else {
				s.normal()
			};
			if bold {
				styled = styled.bold();
			}
			if italic {
				styled = styled.italic();
			}
			if underline {
				styled = styled.underline();
			}
			if strike {
				styled = styled.strikethrough();
			}
			out.push_str(&styled.to_string());
		}
	}

	out
}

/// Map Minecraft named colors (from JSON chat) to `colored::Color`.
fn mc_color_to_colored(name: &str) -> Option<Color> {
	// Handle hex colors: #RRGGBB
	if name.starts_with('#') && name.len() == 7 {
		let r = u8::from_str_radix(&name[1..3], 16).ok()?;
		let g = u8::from_str_radix(&name[3..5], 16).ok()?;
		let b = u8::from_str_radix(&name[5..7], 16).ok()?;
		return Some(Color::TrueColor { r, g, b });
	}

	match name {
		"black" => Some(Color::Black),
		"dark_blue" => Some(Color::Blue),
		"dark_green" => Some(Color::Green),
		"dark_aqua" => Some(Color::Cyan),
		"dark_red" => Some(Color::Red),
		"dark_purple" => Some(Color::Magenta),
		"gold" => Some(Color::TrueColor {
			r: 255,
			g: 170,
			b: 0,
		}),
		"gray" => Some(Color::TrueColor {
			r: 170,
			g: 170,
			b: 170,
		}),
		"dark_gray" => Some(Color::TrueColor {
			r: 85,
			g: 85,
			b: 85,
		}),
		"blue" => Some(Color::TrueColor {
			r: 85,
			g: 85,
			b: 255,
		}),
		"green" => Some(Color::TrueColor {
			r: 85,
			g: 255,
			b: 85,
		}),
		"aqua" => Some(Color::TrueColor {
			r: 85,
			g: 255,
			b: 255,
		}),
		"red" => Some(Color::TrueColor {
			r: 255,
			g: 85,
			b: 85,
		}),
		"light_purple" => Some(Color::TrueColor {
			r: 255,
			g: 85,
			b: 255,
		}),
		"yellow" => Some(Color::TrueColor {
			r: 255,
			g: 255,
			b: 85,
		}),
		"white" => Some(Color::White),
		_ => None,
	}
}
