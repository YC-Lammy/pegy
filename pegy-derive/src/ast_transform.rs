

use crate::ast::*;

impl Alternatives{
    pub fn optimise(&mut self){
        // remove unreachable terms
        for (i, a) in self.alternatives.iter().enumerate(){
            if a.terms.is_empty(){
                self.alternatives.resize_with(i + 1, ||unreachable!());
                break;
            }
        }

        // removes common terms
        if self.alternatives.len() > 1{
            let mut terms_to_remove = 0;
            let terms = self.alternatives[0].terms.len();

            for i in 0..terms{
                let first = &self.alternatives[0];
                let mut equals = 1;

                for a in &self.alternatives[1..]{
                    if let Some(t) = a.terms.get(i){
                        if t.eq(&first.terms[i]){
                            equals += 1;
                        }
                    }
                }

                // term is not equal, break
                if equals != self.alternatives.len(){
                    break;
                }

                // the term is equal
                terms_to_remove += 1;
            }

            for _ in 0..terms_to_remove{
                let first = &mut self.alternatives[0];

                self.preparse.push(first.terms.remove(0));

                for a in &mut self.alternatives[1..]{
                    a.terms.remove(0);
                }
            };
        }
    }
}

impl Alternative{
    pub fn optimise(&mut self){
        
    }
}

#[test]
fn test_preparse(){
    let re = pegy::parse_blocking::<Alternatives, _>("Whitespace* H::<M,u8>::L | Whitespace* $i:\"hello\"");

    let mut alt = re.unwrap();
    alt.optimise();

    assert_eq!(alt, Alternatives{
        // common startings are moved to preparse
        preparse: vec![SpecialTerm::Term(Term::Rule("Whitespace".to_string()), Quantifier::RepeatUnlimited)],
        alternatives: vec![
            Alternative{
                terms: vec![SpecialTerm::Term(Term::Rule("H::<M,u8>::L ".to_string()), Quantifier::None)]
            },
            Alternative{
                terms: vec![SpecialTerm::Binding("i".to_string(), Term::String("hello".to_string()), Quantifier::None)]
            }
        ]
    })
}