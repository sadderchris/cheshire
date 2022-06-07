use pest::Parser;

use crate::scanner::{Rule, SchemeParser};

#[test]
fn parse_boolean_true() {
    let result = SchemeParser::parse(Rule::boolean, "#t");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::boolean, result.as_rule())
}

#[test]
fn parse_boolean_true_alt() {
    let result = SchemeParser::parse(Rule::boolean, "#T");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::boolean, result.as_rule())
}

#[test]
fn parse_boolean_false() {
    let result = SchemeParser::parse(Rule::boolean, "#f");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::boolean, result.as_rule())
}

#[test]
fn parse_boolean_false_alt() {
    let result = SchemeParser::parse(Rule::boolean, "#F");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::boolean, result.as_rule())
}

#[test]
fn parse_character() {
    let result = SchemeParser::parse(Rule::character, r"#\a");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::character, result.as_rule())
}

#[test]
fn parse_character_literal_space() {
    let result = SchemeParser::parse(Rule::character, r"#\ ");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::character, result.as_rule())
}

#[test]
fn parse_character_space() {
    let result = SchemeParser::parse(Rule::character, r"#\space");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::character, result.as_rule())
}

#[test]
fn parse_character_newline() {
    let result = SchemeParser::parse(Rule::character, r"#\newline");
    if let Err(ref parser_error) = result {
        panic!("{}", parser_error);
    }

    let result = result.unwrap().next().unwrap();

    assert_eq!(Rule::character, result.as_rule())
}
