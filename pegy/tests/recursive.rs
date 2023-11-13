use pegy::util::{Boxed, Recursive, ALPHANUMERIC};

type RecursiveName = Recursive<Boxed<Name>>;

#[derive(Debug, Default, PartialEq, Eq, pegy::Parse)]
#[grammar($item0:ALPHANUMERIC $item1:RecursiveName?)]
struct Name(char, Option<Box<Name>>);

#[test]
fn test_recursive_str() {
    let name = pegy::parse_blocking::<Name, _>("ty46ncis");

    assert_eq!(
        name,
        Ok(Name(
            't',
            Some(Box::new(Name(
                'y',
                Some(Box::new(Name(
                    '4',
                    Some(Box::new(Name(
                        '6',
                        Some(Box::new(Name(
                            'n',
                            Some(Box::new(Name(
                                'c',
                                Some(Box::new(Name('i', Some(Box::new(Name('s', None))))))
                            )))
                        )))
                    )))
                )))
            )))
        ))
    )
}
