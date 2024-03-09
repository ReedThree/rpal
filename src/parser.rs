use crate::job::Job;
use crate::pal::PalType;
use rand::{seq::SliceRandom, thread_rng, Rng};
use regex::Regex;

pub enum ParseError {
    UnexpectedEOF(String),
    FormatError(String),
    UnkownInputType(String),
}

impl std::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::UnexpectedEOF(e) => write!(f, "Unexpected EOF: {}", e),
            Self::FormatError(e) => write!(f, "Format error: {}", e),
            Self::UnkownInputType(e) => write!(f, "Unkown config type: {}", e),
        }
    }
}

pub fn parse(pal_type: &PalType, input: &str) -> Result<Vec<Job>, ParseError> {
    match &pal_type {
        PalType::Check => parse_check(input),
        PalType::Pal => parse_pal(input),
        PalType::RandomPal => parse_random_pal(input),
        _ => unreachable!(),
    }
}

fn parse_check(input: &str) -> Result<Vec<Job>, ParseError> {
    let mut result = Vec::new();
    let mut input_lines = input.lines();
    let separator = input_lines.next().ok_or_else(|| {
        ParseError::UnexpectedEOF(String::from("Input ends when reading separator."))
    })?;

    let mut is_output = false;

    let mut this_input = Vec::new();
    let mut this_output = Vec::new();

    let mut this_id = 0;

    for line in input_lines {
        if line != separator {
            // input
            if !is_output {
                let mut line = String::from(line);
                line.push('\n');
                this_input.push(line);
            } else {
                let mut line = String::from(line);
                line.push('\n');
                this_output.push(line);
            }
        } else {
            if !is_output {
                is_output = true;
            } else {
                is_output = false;

                result.push(Job {
                    id: this_id,
                    input: this_input.concat().as_bytes().to_vec(),
                    expected_output: this_output.concat().as_bytes().to_vec(),
                    actual_output: Vec::new(),
                });
                this_id += 1;
                this_input.clear();
                this_output.clear();
            }
        }
    }

    Ok(result)
}

fn parse_pal(input: &str) -> Result<Vec<Job>, ParseError> {
    let mut result = Vec::new();
    let mut input_lines = input.lines();
    let config_type = input_lines.next().ok_or_else(|| {
        ParseError::UnexpectedEOF(String::from("Input ends when reading config type."))
    })?;

    match config_type {
        "simple" => {
            let separator = input_lines.next().ok_or_else(|| {
                ParseError::UnexpectedEOF(String::from("Input ends when reading separator."))
            })?;

            let mut this_input = Vec::new();

            let mut this_id = 0;

            for line in input_lines {
                if line != separator {
                    let mut line = String::from(line);
                    line.push('\n');
                    this_input.push(line);
                } else {
                    result.push(Job {
                        id: this_id,
                        input: this_input.concat().as_bytes().to_vec(),
                        expected_output: Vec::new(),
                        actual_output: Vec::new(),
                    });
                    this_input.clear();
                    this_id += 1;
                }
            }
        }
        "glob" => {
            let mut raw_inputs = Vec::new();
            let separator = input_lines.next().ok_or_else(|| {
                ParseError::UnexpectedEOF(String::from("Input ends when reading separator."))
            })?;
            let mut this_input = Vec::new();

            for line in input_lines {
                if line != separator {
                    let mut line = String::from(line);
                    line.push('\n');
                    this_input.push(line);
                } else {
                    raw_inputs.push(this_input.concat());
                    this_input.clear();
                }
            }

            let mut this_id = 0;

            for raw_input in raw_inputs {
                for expanded_input in expand_glob(raw_input) {
                    result.push(Job {
                        id: this_id,
                        input: expanded_input.as_bytes().to_vec(),
                        expected_output: Vec::new(),
                        actual_output: Vec::new(),
                    });
                    this_id += 1;
                }
            }
        }
        x => return Err(ParseError::UnkownInputType(x.to_string())),
    }
    Ok(result)
}

fn parse_random_pal(input: &str) -> Result<Vec<Job>, ParseError> {
    let mut result = Vec::new();
    let mut input_lines = input.lines();
    let tests_num = input_lines.next().ok_or_else(|| {
        ParseError::UnexpectedEOF(String::from("Input ends when reading tests num."))
    })?;

    let tests_num: usize = tests_num.parse().map_err(|e| {
        ParseError::FormatError(format!("Cannot parse tests num({}): {:?}", tests_num, e))
    })?;

    let mut raw_inputs = Vec::new();
    let separator = input_lines.next().ok_or_else(|| {
        ParseError::UnexpectedEOF(String::from("Input ends when reading separator."))
    })?;
    let mut this_input = Vec::new();

    for line in input_lines {
        if line != separator {
            let mut line = String::from(line);
            line.push('\n');
            this_input.push(line);
        } else {
            raw_inputs.push(this_input.concat());
            this_input.clear();
        }
    }

    let mut this_id = 0;

    let jobs_per_input = tests_num / raw_inputs.len();

    for raw_input in raw_inputs {
        for _ in 0..jobs_per_input {
            result.push(Job {
                id: this_id,
                input: expand_random(raw_input.clone()).as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            });
            this_id += 1;
        }
    }

    Ok(result)
}

pub fn expand_glob(input: String) -> Vec<String> {
    let mut expanded = vec![input];
    loop {
        let before_len = expanded.len();
        expanded = expanded.into_iter().flat_map(|s| expand_one(s)).collect();
        let after_len = expanded.len();

        if before_len == after_len {
            break;
        }
    }

    expanded
}

fn expand_one(input: String) -> Vec<String> {
    let mut result = Vec::new();
    // First capture pattern like: [1-9]
    let first_re = Regex::new(r"\[(\d+)-(\d+)\]").unwrap();
    match first_re.captures(&input) {
        Some(cap) => {
            let full_match = cap
                .get(0)
                .expect("cap[0] is guaranteed to have non-None value")
                .as_str();
            let begin_number: usize = cap
                .get(1)
                .expect("Group 1(Begin number) should exist.")
                .as_str()
                .parse()
                .expect("Group 1(Begin number) should be number.");
            let end_number: usize = cap
                .get(2)
                .expect("Group 2(End number) should exist.")
                .as_str()
                .parse()
                .expect("Group 2(End number) should be number.");
            for i in begin_number..=end_number {
                result.push(input.replace(full_match, &i.to_string()));
            }
        }
        None => {
            // If first pattern not found, try [abc]、[abc123kkk]
            let then_re = Regex::new(r"[^\\](\[(.+?[^\\])\])").unwrap();
            match then_re.captures(&input) {
                Some(cap) => {
                    let full_match = cap
                        .get(1)
                        .expect("Group 1(Full match) should exist.")
                        .as_str();
                    let inner = cap.get(2).expect("Group 2(Inner) should exist.").as_str();

                    inner
                        .chars()
                        .into_iter()
                        .filter(|c| *c != '\\')
                        .for_each(|c| result.push(input.replace(full_match, &c.to_string())));
                }
                None => {
                    result.push(input);
                }
            }
        }
    }
    result
}

fn expand_random(mut input: String) -> String {
    // First capture pattern like: [1-9]
    let first_re = Regex::new(r"\[(\d+)-(\d+)\]").unwrap();
    let _input = input.clone();
    first_re.captures_iter(&_input).for_each(|cap| {
        let full_match = cap
            .get(0)
            .expect("cap[0] is guaranteed to have non-None value")
            .as_str();
        let begin_number: usize = cap
            .get(1)
            .expect("Group 1(Begin number) should exist.")
            .as_str()
            .parse()
            .expect("Group 1(Begin number) should be number.");
        let end_number: usize = cap
            .get(2)
            .expect("Group 2(End number) should exist.")
            .as_str()
            .parse()
            .expect("Group 2(End number) should be number.");
        input = input.replace(
            full_match,
            &thread_rng()
                .gen_range(begin_number..=end_number)
                .to_string(),
        );
    });

    // then [abc]、[abc123kkk]
    let then_re = Regex::new(r"[^\\](\[(.+?[^\\])\])").unwrap();
    let _input = input.clone();
    then_re.captures_iter(&_input).for_each(|cap| {
        let full_match = cap
            .get(1)
            .expect("Group 1(Full match) should exist.")
            .as_str();
        let inner = cap.get(2).expect("Group 2(Inner) should exist.").as_str();

        let inner_list: Vec<char> = inner.chars().into_iter().filter(|c| *c != '\\').collect();

        let choosed = inner_list
            .choose(&mut thread_rng())
            .expect("Inner list length should > 0");

        input = input.replace(full_match, &choosed.to_string());
    });
    input
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_check() {
        let input = "----\naaabbbcccddd\neeefffggghhh\n----\naaabbbcccddd\neeefffggghhh\n----\naaabbbcccddd\neeefffggghhh\n----\naaabbbcccddd\neeefffggghhh\n----";
        let pal_list = parse(&PalType::Check, input).unwrap();
        assert_eq!(
            pal_list[0],
            Job {
                id: 0,
                input: "aaabbbcccddd\neeefffggghhh\n".as_bytes().to_vec(),
                expected_output: "aaabbbcccddd\neeefffggghhh\n".as_bytes().to_vec(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[1],
            Job {
                id: 1,
                input: "aaabbbcccddd\neeefffggghhh\n".as_bytes().to_vec(),
                expected_output: "aaabbbcccddd\neeefffggghhh\n".as_bytes().to_vec(),
                actual_output: Vec::new(),
            }
        );
    }

    #[test]
    fn test_parse_pal_simple() {
        let input = "simple\n----\naaabbb\ncccddd\n----\neeefff\nggghhh\n----\n";
        let pal_list = parse(&PalType::Pal, input).unwrap();
        assert_eq!(
            pal_list[0],
            Job {
                id: 0,
                input: "aaabbb\ncccddd\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[1],
            Job {
                id: 1,
                input: "eeefff\nggghhh\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
    }

    #[test]
    fn test_expand_glob_simple() {
        let raw_input = String::from("[1-3]bc[abc145]");
        let output = expand_glob(raw_input);

        assert_eq!(
            output,
            vec![
                String::from("1bca"),
                String::from("1bcb"),
                String::from("1bcc"),
                String::from("1bc1"),
                String::from("1bc4"),
                String::from("1bc5"),
                String::from("2bca"),
                String::from("2bcb"),
                String::from("2bcc"),
                String::from("2bc1"),
                String::from("2bc4"),
                String::from("2bc5"),
                String::from("3bca"),
                String::from("3bcb"),
                String::from("3bcc"),
                String::from("3bc1"),
                String::from("3bc4"),
                String::from("3bc5"),
            ]
        )
    }

    #[test]
    fn test_expand_glob_do_not_touch_escape_character() {
        let raw_input = String::from(r"[1-3]abc\[1-3\]kkk\[abc\]");
        let output = expand_glob(raw_input);

        assert_eq!(
            output,
            vec![
                String::from(r"1abc\[1-3\]kkk\[abc\]"),
                String::from(r"2abc\[1-3\]kkk\[abc\]"),
                String::from(r"3abc\[1-3\]kkk\[abc\]"),
            ]
        )
    }

    #[test]
    fn test_expand_glob_escaped_dash() {
        let raw_input = String::from(r"[1-3]abc[123\-456]");
        let output = expand_glob(raw_input);

        assert_eq!(
            output,
            vec![
                String::from(r"1abc1"),
                String::from(r"1abc2"),
                String::from(r"1abc3"),
                String::from(r"1abc-"),
                String::from(r"1abc4"),
                String::from(r"1abc5"),
                String::from(r"1abc6"),
                String::from(r"2abc1"),
                String::from(r"2abc2"),
                String::from(r"2abc3"),
                String::from(r"2abc-"),
                String::from(r"2abc4"),
                String::from(r"2abc5"),
                String::from(r"2abc6"),
                String::from(r"3abc1"),
                String::from(r"3abc2"),
                String::from(r"3abc3"),
                String::from(r"3abc-"),
                String::from(r"3abc4"),
                String::from(r"3abc5"),
                String::from(r"3abc6"),
            ]
        )
    }

    #[test]
    fn test_parse_pal() {
        // simple
        let input = "simple\n----\n1\n2\n----\n3\n4\n----\n";
        let pal_list = parse(&PalType::Pal, input).unwrap();
        assert_eq!(
            pal_list[0],
            Job {
                id: 0,
                input: "1\n2\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[1],
            Job {
                id: 1,
                input: "3\n4\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );

        // glob
        let input = "glob\n----\n[1-3]abc\n----\nkkk[1-3]\n----\n";
        let pal_list = parse(&PalType::Pal, input).unwrap();
        assert_eq!(
            pal_list[0],
            Job {
                id: 0,
                input: "1abc\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[1],
            Job {
                id: 1,
                input: "2abc\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[2],
            Job {
                id: 2,
                input: "3abc\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[3],
            Job {
                id: 3,
                input: "kkk1\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[4],
            Job {
                id: 4,
                input: "kkk2\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
        assert_eq!(
            pal_list[5],
            Job {
                id: 5,
                input: "kkk3\n".as_bytes().to_vec(),
                expected_output: Vec::new(),
                actual_output: Vec::new(),
            }
        );
    }
}
