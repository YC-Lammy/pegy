use core::marker::PhantomData;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{Error, Source, Span};

#[derive(Debug)]
pub enum Node<N, T> {
    Primary(T),
    Expr {
        rule: N,
        left: Option<Box<Node<N, T>>>,
        right: Option<Box<Node<N, T>>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrattToken {
    Preffix(String),
    Suffix(String),
    Inffix(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token {
    Prefix(usize),
    Suffix(usize),
    Inffix(usize),
}

enum ParsedToken<T> {
    /// the index of prefix
    Preffix(usize, Span),
    Suffix(usize, Span),
    Inffix(usize, Span),
    Primary(T),
    Node {
        rule: Token,
        left: Option<Box<ParsedToken<T>>>,
        right: Option<Box<ParsedToken<T>>>,
    },
}

struct Rule<N> {
    name: N,
    level: usize,
    token: Token,
}

pub struct PrattBuilder<N: Clone, T> {
    /// ordered by their precedence
    rules: Vec<Rule<N>>,
    /// ordered by their precedence
    prefixes: Vec<(usize, String)>,
    /// ordered by their precedence
    suffixes: Vec<(usize, String)>,
    /// ordered by their precedence
    inffixes: Vec<(usize, String)>,

    id: usize,

    _mark: PhantomData<T>,
}

impl<N: Clone, T: crate::Parse> PrattBuilder<N, T> {
    pub fn new() -> PrattBuilder<N, T> {
        PrattBuilder {
            rules: Vec::new(),
            prefixes: Vec::new(),
            suffixes: Vec::new(),
            inffixes: Vec::new(),

            id: 0,

            _mark: PhantomData,
        }
    }

    pub fn with_rule(mut self, precedence: usize, name: N, token: PrattToken) -> Self {
        let id = self.id;
        self.id += 1;

        let token = match token {
            PrattToken::Inffix(s) => {
                self.inffixes.push((id, s));
                Token::Inffix(id)
            }
            PrattToken::Preffix(s) => {
                self.prefixes.push((id, s));
                Token::Prefix(id)
            }
            PrattToken::Suffix(s) => {
                self.suffixes.push((id, s));
                Token::Suffix(id)
            }
        };

        self.rules.push(Rule {
            name: name,
            level: precedence,
            token: token,
        });

        return self;
    }

    pub fn build(mut self) -> Pratt<N, T> {
        self.rules.sort_by(|a, b| a.level.cmp(&b.level));

        Pratt {
            rules: self.rules,
            prefixes: self.prefixes,
            suffixes: self.suffixes,
            inffixes: self.inffixes,
            _mark: PhantomData,
        }
    }
}

pub struct Pratt<N: Clone, T: crate::Parse> {
    /// ordered by their precedence
    rules: Vec<Rule<N>>,
    /// ordered by their precedence
    prefixes: Vec<(usize, String)>,
    /// ordered by their precedence
    suffixes: Vec<(usize, String)>,
    /// ordered by their precedence
    inffixes: Vec<(usize, String)>,

    _mark: PhantomData<T>,
}

impl<N: Clone, T: crate::Parse> Pratt<N, T> {
    pub fn builder() -> PrattBuilder<N, T> {
        PrattBuilder::new()
    }

    async fn parse_tokens<S: Source>(
        &self,
        src: &mut S,
    ) -> Result<Vec<ParsedToken<T::Output>>, Error> {
        let mut tokens = Vec::new();

        'prefix: loop {
            for (id, p) in self.prefixes.iter().rev() {
                let start = src.current_position();
                if src.match_str(p).await {
                    let end = src.current_position();
                    tokens.push(ParsedToken::Preffix(*id, Span::new(start, end)));
                    continue 'prefix;
                }
            }
            break;
        }

        let prim = T::parse(src).await?;
        tokens.push(ParsedToken::Primary(prim));

        'suffix: loop {
            for (id, s) in self.suffixes.iter().rev() {
                let start = src.current_position();
                if src.match_str(s).await {
                    tokens.push(ParsedToken::Suffix(
                        *id,
                        Span::new(start, src.current_position()),
                    ));
                    continue 'suffix;
                }
            }
            break;
        }

        // repeat inffix -> preffix -> primary -> suffix
        loop {
            // only match one inffix
            let mut has_infix = false;
            for (id, i) in self.inffixes.iter().rev() {
                let start = src.current_position();
                if src.match_str(i).await {
                    has_infix = true;
                    tokens.push(ParsedToken::Inffix(
                        *id,
                        Span::new(start, src.current_position()),
                    ));
                    break;
                }
            }

            // must have infix before next primary expression
            if !has_infix {
                break;
            }

            // parse prefixes
            'prefix: loop {
                for (id, p) in self.prefixes.iter().rev() {
                    let start = src.current_position();
                    if src.match_str(p).await {
                        let end = src.current_position();
                        tokens.push(ParsedToken::Preffix(*id, Span::new(start, end)));
                        continue 'prefix;
                    }
                }
                break;
            }

            // parse primary expression
            match T::parse(src).await {
                Ok(prim) => {
                    tokens.push(ParsedToken::Primary(prim));
                }
                Err(_) => {
                    break;
                }
            };

            'suffix: loop {
                for (id, s) in self.suffixes.iter().rev() {
                    let start = src.current_position();
                    if src.match_str(s).await {
                        tokens.push(ParsedToken::Suffix(
                            *id,
                            Span::new(start, src.current_position()),
                        ));
                        continue 'suffix;
                    }
                }
                break;
            }
        }

        return Ok(tokens);
    }

    async fn process_parsed_tokens(
        &self,
        mut tokens: Vec<ParsedToken<T::Output>>,
    ) -> Result<ParsedToken<T::Output>, Error> {
        for rules in self.rules.iter().rev() {
            match rules.token {
                // prefix tokens matches from back to forth
                Token::Prefix(prefix_id) => {
                    // reverse iter
                    for token_idx in (0..tokens.len()).rev() {
                        // get token
                        let token = &tokens[token_idx];

                        // a prefix token
                        if let ParsedToken::Preffix(id, span) = token {
                            // next token if not id
                            if prefix_id != *id {
                                continue;
                            }

                            // check right
                            if let Some(ParsedToken::Primary(_)) = tokens.get(token_idx + 1) {
                            } else if let Some(ParsedToken::Node { .. }) = tokens.get(token_idx + 1)
                            {
                            } else {
                                return Err(Error::new(*span, "unexpected token"));
                            }

                            // copy id
                            let id = *id;

                            // remove right hand side
                            let right = tokens.remove(token_idx + 1);

                            // replace self by a node
                            tokens[token_idx] = ParsedToken::Node {
                                rule: Token::Prefix(id),
                                left: None,
                                right: Some(Box::new(right)),
                            };
                        } else {
                            // not a prefix
                            continue;
                        };
                    }

                    // continue to next loop
                    continue;
                }
                // others matches in order
                _ => {}
            }

            let mut token_idx = 0;
            while token_idx < tokens.len() {
                let token = &tokens[token_idx];

                match token {
                    ParsedToken::Inffix(id, span) => {
                        // match the rule id
                        if let Token::Inffix(rule_id) = rules.token {
                            if rule_id != *id {
                                token_idx += 1;
                                continue;
                            }
                        } else {
                            token_idx += 1;
                            continue;
                        }

                        // check left
                        if let Some(ParsedToken::Primary(_)) = tokens.get(token_idx - 1) {
                        } else if let Some(ParsedToken::Node { .. }) = tokens.get(token_idx - 1) {
                        } else {
                            return Err(Error::new(*span, "unexpected token"));
                        }

                        // check right
                        if let Some(ParsedToken::Primary(_)) = tokens.get(token_idx + 1) {
                        } else if let Some(ParsedToken::Node { .. }) = tokens.get(token_idx + 1) {
                        } else {
                            return Err(Error::new(*span, "unexpected token"));
                        }

                        let id = *id;

                        // remove right first
                        let right = tokens.remove(token_idx + 1);
                        let left = tokens.remove(token_idx - 1);

                        // because left has been removed, index shifts by one
                        token_idx -= 1;

                        tokens[token_idx] = ParsedToken::Node {
                            rule: Token::Inffix(id),
                            left: Some(Box::new(left)),
                            right: Some(Box::new(right)),
                        };
                    }
                    ParsedToken::Suffix(id, span) => {
                        if let Token::Suffix(sufix_id) = rules.token {
                            if sufix_id != *id {
                                token_idx += 1;
                                continue;
                            }
                        } else {
                            token_idx += 1;
                            continue;
                        }

                        // check left
                        if let Some(ParsedToken::Primary(_)) = tokens.get(token_idx - 1) {
                        } else if let Some(ParsedToken::Node { .. }) = tokens.get(token_idx - 1) {
                        } else {
                            return Err(Error::new(*span, "unexpected token"));
                        }

                        let id = *id;

                        let left = tokens.remove(token_idx - 1);

                        // because left has been removed, index shifts by one
                        token_idx -= 1;

                        tokens[token_idx] = ParsedToken::Node {
                            rule: Token::Suffix(id),
                            left: Some(Box::new(left)),
                            right: None,
                        };
                    }
                    // skip primary and node
                    _ => {}
                };

                token_idx += 1;
            }
        }

        // at this point, all rules has been parsed

        // duplicated primary expressions
        if tokens.len() != 0 {}

        return Ok(tokens.remove(0));
    }

    pub async fn parse<S: Source>(&self, src: &mut S) -> Result<Node<N, T::Output>, Error> {
        let tokens = self.parse_tokens(src).await?;
        let node = self.process_parsed_tokens(tokens).await?;

        return Ok(self.parsed_token_to_node(node));
    }

    fn parsed_token_to_node(&self, token: ParsedToken<T::Output>) -> Node<N, T::Output> {
        match token {
            ParsedToken::Primary(p) => return Node::Primary(p),
            ParsedToken::Node { rule, left, right } => {
                let left = if let Some(left) = left {
                    Some(Box::new(self.parsed_token_to_node(*left)))
                } else {
                    None
                };
                let right = if let Some(right) = right {
                    Some(Box::new(self.parsed_token_to_node(*right)))
                } else {
                    None
                };

                if let Some(rule) = self.rules.iter().find(|r| r.token == rule) {
                    return Node::Expr {
                        rule: rule.name.clone(),
                        left: left,
                        right: right,
                    };
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        }
    }
}
