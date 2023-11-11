use pegy::pratt::{PrattBuilder, PrattToken};
use pegy::StrSource;

fn main() {
    #[derive(Debug, Clone, Copy)]
    enum Rule {
        Add,
        Sub,
        Mul,
        Div,
        Increment,
        Neg,
    }

    let pratt = PrattBuilder::<Rule, usize>::new()
        .with_rule(1, Rule::Add, PrattToken::Inffix("+".to_string()))
        .with_rule(1, Rule::Sub, PrattToken::Inffix("-".to_string()))
        .with_rule(2, Rule::Mul, PrattToken::Inffix("*".to_string()))
        .with_rule(2, Rule::Div, PrattToken::Inffix("/".to_string()))
        .with_rule(3, Rule::Neg, PrattToken::Preffix("-".to_string()))
        .with_rule(4, Rule::Increment, PrattToken::Suffix("++".to_string()))
        .build();

    let mut src = StrSource::new("99/66+77*-4++");

    let re = futures::executor::block_on(pratt.parse(&mut src));

    println!("{:#?}", re);
}
