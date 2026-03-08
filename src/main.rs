#![windows_subsystem = "windows"]

use eframe::egui;

use rust_decimal::prelude::*;

fn main() -> eframe::Result<()> {
    let options: eframe::NativeOptions = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([670.0, 750.0])
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native(
        "EguiDentaku",
        options,
        Box::new(|_cc: &eframe::CreationContext<'_>| Ok(Box::new(AppWindow::default()))),
    )
}

struct AppWindow {
    input: String,
    result: String,
}

impl AppWindow {
    fn display_input(&mut self, s: &str) {
        let is_head_zero = self.input.starts_with("0");
        let is_first_input_operator = self.input.chars().nth(1).is_none()
            && (s.parse::<u8>().is_ok() || s == "(" || s == ")" || s == "-");
        if is_head_zero && is_first_input_operator {
            self.input = s.to_string();
        } else {
            self.input.push_str(s);
            self.input = self.input.formalize();
        }
    }

    fn calculate(&mut self) {
        let is_valid_expr = Self::check_validity(&self.input);
        let mut engine = CalcEngine {
            input: self.input.formalize(),
            tokens: Vec::new(),
            result: Some(Decimal::new(0, 0)),
        };
        engine = engine.operator_convert().to_rpn();

        if engine.result.is_none() || !is_valid_expr {
            self.result = "Error".to_string();
            return;
        }
        if let Some(res) = engine.eval_rpn() {
            self.result = res.to_string();
        } else {
            self.result = "Error".to_string();
        }
    }

    fn reset(&mut self) {
        self.input = "0".to_string();
        self.result = "".to_string();
    }

    fn check_validity(s: &str) -> bool {
        let mut is_valid = true;
        if s.contains("÷0") {
            is_valid = false;
        }
        is_valid
    }
}
impl Default for AppWindow {
    fn default() -> Self {
        Self {
            input: "0".to_string(),
            result: "".to_string(),
        }
    }
}

struct CalcEngine {
    input: String,
    tokens: Vec<String>,
    result: Option<Decimal>,
}

impl CalcEngine {
    const fn get_prec(op: char) -> u8 {
        match op {
            '+' | '-' => 2,
            '*' | '/' => 3,
            _ => 0,
        }
    }

    fn eval_rpn(self) -> Option<Decimal> {
        let tokens = &self.tokens;
        const OPERATORS: [&str; 4] = ["+", "-", "*", "/"];
        let mut stack: Vec<Decimal> = Vec::new();
        for token in tokens {
            if OPERATORS.contains(&token.as_str()) {
                let b: Decimal = stack.pop()?;
                let a: Decimal = stack.pop()?;
                if token == "/" && b == Decimal::ZERO {
                    return None;
                }
                match token.as_str() {
                    "+" => stack.push(a.checked_add(b)?),
                    "-" => stack.push(a.checked_sub(b)?),
                    "*" => stack.push(a.checked_mul(b)?),
                    "/" => stack.push(a.checked_div(b)?),
                    _ => {
                        return None;
                    }
                }
            } else {
                stack.push(token.parse::<Decimal>().ok()?);
            }
        }

        stack
            .pop()
            .map(|v: Decimal| v.round_dp_with_strategy(10, RoundingStrategy::MidpointAwayFromZero))
    }

    fn find_invalid_dot(s: &str) -> Result<(), ()> {
        const OPERATORS: [&str; 4] = ["+", "-", "×", "÷"];
        let tokens = s.to_string().tokenize();
        tokens.iter().try_for_each(|token| {
            if !OPERATORS.contains(&token.as_str()) {
                token.parse::<f64>().map(|_| ()).map_err(|_| ())
            } else {
                Ok(())
            }
        })
    }
}

trait Evaluable {
    fn to_rpn(self) -> Self;
    fn operator_convert(self) -> Self;
}

impl Evaluable for CalcEngine {
    fn to_rpn(self) -> Self {
        let tokens = &self.tokens;
        const OPERATORS: [&str; 4] = ["+", "-", "*", "/"];
        let mut output: Vec<String> = Vec::new();
        let mut stack: Vec<String> = Vec::new();

        for token in tokens {
            let is_token_left_parenthesis: bool = token == "(";
            let is_token_right_parenthesis: bool = token == ")";

            if OPERATORS.contains(&token.as_str()) {
                while let Some(top) = stack.last() {
                    if Self::get_prec(top.chars().next().unwrap())
                        >= Self::get_prec(token.chars().next().unwrap())
                    {
                        output.push(stack.pop().unwrap());
                    } else {
                        break;
                    }
                }
                stack.push(token.to_string());
            } else if is_token_left_parenthesis {
                stack.push(token.to_string())
            } else if is_token_right_parenthesis {
                while let Some(top) = stack.last() {
                    if !stack.iter().any(|x| x == "(") {
                        return Self {
                            result: None,
                            ..self
                        };
                    }

                    if top != "(" {
                        output.push(stack.pop().unwrap());
                    } else {
                        stack.pop();
                        break;
                    }
                }
            } else {
                output.push(token.to_string());
            }
        }
        while let Some(top) = stack.pop() {
            if top == "(" || top == ")" {
                return Self {
                    result: None,
                    ..self
                };
            }
            output.push(top);
        }
        Self {
            tokens: output,
            ..self
        }
    }

    fn operator_convert(self) -> Self {
        let mut iter = self.input.tokenize().clone();
        iter = iter
            .iter_mut()
            .map(|c| match c.as_str() {
                "×" => "*".to_string(),
                "÷" => "/".to_string(),
                _ => c.to_string(),
            })
            .collect::<Vec<String>>();
        Self {
            tokens: iter,
            ..self
        }
    }
}
trait Formalizer {
    fn formalize(&self) -> String;
    fn sign_invert(&self) -> String;
}

impl Formalizer for String {
    fn formalize(&self) -> String {
        let mut result = String::new();

        const OPERATORS: [char; 4] = ['+', '-', '×', '÷'];

        for c in self.chars() {
            match c {
                op if OPERATORS.contains(&op) => {
                    if let Some(l) = result.chars().last() {
                        if OPERATORS.contains(&l) {
                            result.pop();
                        } else if l == '.' {
                            continue;
                        }
                    }
                    result.push(op);
                }

                '.' => {
                    let potential_new_string = format!("{}{}", result, c);
                    if CalcEngine::find_invalid_dot(&potential_new_string).is_ok() {
                        result.push(c);
                    }
                }

                _ => result.push(c),
            }
        }

        if result.len() == 1
            && let Some(c) = result.chars().next()
            && OPERATORS.contains(&c)
        {
            result = format!("{}{}", "0", c)
        }
        result
    }

    fn sign_invert(&self) -> String {
        const OPERATORS: [&str; 4] = ["+", "-", "×", "÷"];
        let mut result = self.tokenize();
        let t = result
            .iter_mut()
            .rev()
            .find(|s| OPERATORS.contains(&s.as_str()));

        if let Some(s) = t {
            match s.as_str() {
                "+" => *s = "-".to_string(),
                "-" => *s = "+".to_string(),
                _ => {}
            }
        }

        result.join("")
    }
}

trait Tokenizer {
    fn tokenize(&self) -> Vec<String>;
}

impl Tokenizer for String {
    fn tokenize(&self) -> Vec<String> {
        const OPERATORS: [char; 4] = ['+', '-', '×', '÷'];
        let mut tmp = String::new();
        let mut result = Vec::new();
        let mut is_negative_num: bool = false;
        let mut chars = self.chars().peekable();
        let mut i = 0;
        while let Some(c) = chars.next() {
            if c.is_ascii_digit() || c == '.' || is_negative_num || (c == '-' && i == 0) {
                tmp.push(c);
            } else if (OPERATORS.contains(&c) && !is_negative_num) || c == '(' || c == ')' {
                if !tmp.is_empty() {
                    result.push(tmp);
                    tmp = String::new();
                }
                result.push(c.to_string());
            }
            is_negative_num =
                !c.is_ascii_digit() && c != ')' && chars.peek().is_some_and(|c| *c == '-');

            if c.is_ascii_digit() && chars.peek().is_some_and(|c| *c == '(') {
                result.push("×".to_string());
            }
            i += 1;
        }
        if !tmp.is_empty() {
            result.push(tmp);
        }

        result
    }
}

impl eframe::App for AppWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ctx.set_pixels_per_point(3.0);
            ui.horizontal(|ui| {
                egui::Grid::new("calc_buttons")
                    .spacing([1.0, 1.0])
                    .show(ui, |ui| {
                        let buttons: [[&str; 4]; 5] = [
                            ["(", ")", "C", "Del"],
                            ["7", "8", "9", "÷"],
                            ["4", "5", "6", "×"],
                            ["1", "2", "3", "-"],
                            ["+/-", "0", ".", "+"],
                        ];
                        for row in buttons {
                            for label in row {
                                if ui
                                    .add_sized([50.0, 40.0], egui::Button::new(label))
                                    .clicked()
                                {
                                    match label {
                                        "C" => self.reset(),
                                        "Del" => match self.input.len() {
                                            1 => self.input = "0".to_string(),
                                            _ => {
                                                self.input.pop();
                                            }
                                        },
                                        "+/-" => self.input = self.input.sign_invert(),
                                        _ => self.display_input(label),
                                    }
                                }
                            }
                            ui.end_row();
                        }
                    });

                if ui
                    .add_sized([50.0, 204.0], egui::Button::new("="))
                    .clicked()
                {
                    self.calculate();
                }
            });

            let available = ui.available_width();
            let len = self.input.len() as f32;

            let size = (available / len * 1.8).clamp(10.0, 20.0);

            ui.label(egui::RichText::new(&self.input).size(size));
            ui.label(egui::RichText::new(format!("= {}", self.result)).size(size));
        });
    }
}
